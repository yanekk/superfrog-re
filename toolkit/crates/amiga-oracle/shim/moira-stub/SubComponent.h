// Stands in for vAmiga's SubComponent when Moira is compiled alone for the routine runner
// (moira_shim.cpp): Moira derives from it and passes it the Amiga, which it never uses.
#pragma once

namespace vamiga {

class Amiga;

class SubComponent {
public:
    explicit SubComponent(Amiga &) { }
    virtual ~SubComponent() = default;
};

}
