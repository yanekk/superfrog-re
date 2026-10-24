"""Pull a model redraw back onto the original's grid.

The model keeps the layout of what it redraws but not its geometry: content comes back
scaled by a percent or two and drifts 1-3 original pixels locally (measured on the style
tests, 2026-10-24). One global x/y scale + shift is not enough, so after it we match
blocks of edges against the original and warp by a smooth displacement field.

numpy only: the repo's tools do not depend on scipy or OpenCV.
"""
import numpy as np
from PIL import Image

W = 2   # matching works at 2x the original's resolution


def gauss1d(sigma):
    r = max(1, int(3 * sigma))
    k = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
    return k / k.sum()


def blur(a, sigma):
    if sigma <= 0:
        return a
    k = gauss1d(sigma)
    r = len(k) // 2
    p = np.pad(a, [(r, r), (r, r)] + [(0, 0)] * (a.ndim - 2), mode='edge')
    p = sum(k[i] * p[i:i + a.shape[0]] for i in range(len(k)))
    return sum(k[i] * p[:, i:i + a.shape[1]] for i in range(len(k)))


def features(rgb):
    """Edge-strength image used for matching: blurred gradient magnitude of luma."""
    g = rgb.astype(float) @ [0.299, 0.587, 0.114]
    g = blur(g, 1.0)
    gy, gx = np.gradient(g)
    e = blur(np.hypot(gx, gy), 1.0)
    return (e - e.mean()) / (e.std() + 1e-9)


def ncc(a, b):
    a = a - a.mean(); b = b - b.mean()
    return float((a * b).sum() / (np.sqrt((a * a).sum() * (b * b).sum()) + 1e-9))


def sample(img, sy, sx):
    """Bilinear sample of img (H, W[, C]) at float coords (clamped)."""
    h, w = img.shape[:2]
    sx = np.clip(sx, 0, w - 1.001); sy = np.clip(sy, 0, h - 1.001)
    x0 = sx.astype(int); y0 = sy.astype(int)
    fx = sx - x0; fy = sy - y0
    if img.ndim == 3:
        fx = fx[..., None]; fy = fy[..., None]
    a = img[y0, x0]; b = img[y0, x0 + 1]; c = img[y0 + 1, x0]; d = img[y0 + 1, x0 + 1]
    return (a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy


def global_fit(R, O):
    """Best (sx, sy, dx, dy) with O[y*sy + dy, x*sx + dx] ~ R[y, x] (both at matching res)."""
    h, w = R.shape
    yy, xx = np.mgrid[0:h:4, 0:w:4].astype(float)
    Rs = R[::4, ::4]
    best = (-9, 1, 1, 0, 0)
    for sx in np.linspace(0.97, 1.03, 7):
        for sy in np.linspace(0.97, 1.03, 7):
            for dy in range(-12, 13, 4):
                for dx in range(-12, 13, 4):
                    v = ncc(sample(O, yy * sy + dy, xx * sx + dx), Rs)
                    if v > best[0]:
                        best = (v, sx, sy, dx, dy)
    # refine scale and shift around the coarse optimum
    yy, xx = np.mgrid[0:h:2, 0:w:2].astype(float)
    Rs = R[::2, ::2]
    v0, sx0, sy0, dx0, dy0 = best
    best = (-9,) + best[1:]
    for sx in np.linspace(sx0 - 0.005, sx0 + 0.005, 3):
        for sy in np.linspace(sy0 - 0.005, sy0 + 0.005, 3):
            for dy in range(dy0 - 3, dy0 + 4):
                for dx in range(dx0 - 3, dx0 + 4):
                    v = ncc(sample(O, yy * sy + dy, xx * sx + dx), Rs)
                    if v > best[0]:
                        best = (v, sx, sy, dx, dy)
    return best


def block_sums(a, ys, xs, bs):
    """Sum of a over every bs x bs block at (ys[i], xs[j]), via an integral image."""
    I = np.zeros((a.shape[0] + 1, a.shape[1] + 1))
    I[1:, 1:] = a.cumsum(0).cumsum(1)
    Y = np.array(ys)[:, None]; X = np.array(xs)[None, :]
    return I[Y + bs, X + bs] - I[Y, X + bs] - I[Y + bs, X] + I[Y, X]


def local_field(R, Og, bs=40, step=20, r=8):
    """Displacement (dy, dx) per block of Og (already globally fitted) onto R, plus confidence.

    Every block is scored for one shift at a time over the whole image (integral images),
    which is ~100x faster than looping blocks in Python.
    """
    h, w = R.shape
    ys = list(range(0, h - bs + 1, step)); xs = list(range(0, w - bs + 1, step))
    n = bs * bs
    sR = block_sums(R, ys, xs, bs); sRR = block_sums(R * R, ys, xs, bs)
    varR = sRR - sR * sR / n
    P = np.pad(Og, r, mode='edge')
    best = np.full((len(ys), len(xs)), -9.0); D = np.zeros((len(ys), len(xs), 2))
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            O = P[r + dy:r + dy + h, r + dx:r + dx + w]
            sO = block_sums(O, ys, xs, bs); sOO = block_sums(O * O, ys, xs, bs)
            sRO = block_sums(R * O, ys, xs, bs)
            v = (sRO - sR * sO / n) / (np.sqrt(np.maximum(varR * (sOO - sO * sO / n), 1e-9)))
            m = v > best
            best[m] = v[m]; D[m] = (dy, dx)
    std = np.sqrt(np.maximum(varR / n, 0))
    C = np.where(std < 0.25, 0.0, np.maximum(best, 0) * np.minimum(1.0, std))   # flat sky: no evidence
    return ys, xs, D, C, bs


def smooth_field(ys, xs, D, C, bs, shape, sigma=1.5):
    """Confidence-weighted smoothing on the block lattice, then bilinear to full res."""
    # Robust: drop blocks that disagree with their neighbourhood median by > 3 px.
    med = np.zeros_like(D)
    n, m = C.shape
    for i in range(n):
        for j in range(m):
            sl = (slice(max(0, i - 2), i + 3), slice(max(0, j - 2), j + 3))
            wts = C[sl].ravel(); good = wts > 0
            if good.any():
                for k in range(2):
                    med[i, j, k] = np.median(D[sl][..., k].ravel()[good])
    bad = np.hypot(*(D - med).transpose(2, 0, 1)) > 3
    Cw = np.where(bad, 0, C)
    num = np.stack([blur(D[..., k] * Cw, sigma) for k in range(2)], -1)
    den = blur(Cw, sigma)[..., None]
    F = np.where(den > 1e-3, num / np.maximum(den, 1e-9), 0)
    # Blocks with no evidence at all fall back to the overall weighted mean.
    if Cw.sum() > 0:
        mean = (D * Cw[..., None]).sum((0, 1)) / Cw.sum()
        far = den[..., 0] < 0.05
        F[far] = mean
    h, w = shape
    cy = np.array(ys) + bs / 2; cx = np.array(xs) + bs / 2
    gy = np.interp(np.arange(h), cy, np.arange(len(ys)))
    gx = np.interp(np.arange(w), cx, np.arange(len(xs)))
    GY, GX = np.meshgrid(gy, gx, indexing='ij')
    return sample(F, GY, GX)          # (h, w, 2) in matching pixels


def register(orig_rgb, out_rgb, scale=4):
    """Warp out_rgb (model output, any size) onto orig_rgb's grid at `scale`x.

    Returns (warped (H*scale, W*scale, 3) float, info dict).
    """
    oh, ow = orig_rgb.shape[:2]
    R = features(np.repeat(np.repeat(orig_rgb, W, 0), W, 1))
    Om = np.asarray(Image.fromarray(out_rgb).resize((ow * W, oh * W), Image.LANCZOS))
    O = features(Om)
    v, sx, sy, dx, dy = global_fit(R, O)
    yy, xx = np.mgrid[0:oh * W, 0:ow * W].astype(float)
    Og = sample(O, yy * sy + dy, xx * sx + dx)
    ys, xs, D, C, bs = local_field(R, Og)
    F = smooth_field(ys, xs, D, C, bs, R.shape)
    # Map: target pixel (at `scale`x) -> source pixel in the full-size model output.
    H, Wd = oh * scale, ow * scale
    ty, tx = np.mgrid[0:H, 0:Wd].astype(float)
    my = (ty + 0.5) * W / scale - 0.5; mx = (tx + 0.5) * W / scale - 0.5      # matching coords
    f = sample(F, my, mx)
    sy_m = (my + f[..., 0]) * sy + dy; sx_m = (mx + f[..., 1]) * sx + dx        # in O (matching res)
    ky = out_rgb.shape[0] / (oh * W); kx = out_rgb.shape[1] / (ow * W)
    warped = sample(out_rgb.astype(float), (sy_m + 0.5) * ky - 0.5, (sx_m + 0.5) * kx - 0.5)
    after = ncc(features(np.asarray(Image.fromarray(warped.clip(0, 255).astype(np.uint8)).resize(
        (ow * W, oh * W), Image.LANCZOS))), R)
    drift = np.hypot(D[..., 0], D[..., 1])[C > 0]
    info = dict(global_ncc=round(v, 3), sx=round(sx, 4), sy=round(sy, 4), dx=dx, dy=dy,
                drift_median=float(np.median(drift)) / W if drift.size else 0.0,
                drift_p90=float(np.percentile(drift, 90)) / W if drift.size else 0.0,
                after_ncc=round(after, 3))
    return warped, info
