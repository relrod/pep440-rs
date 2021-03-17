#!/usr/bin/env python

import itertools

# This is all taken from pypa/packaging, test_version.py

VERSIONS = [
    # Implicit epoch of 0
    "1.0.dev456",
    "1.0a1",
    "1.0a2.dev456",
    "1.0a12.dev456",
    "1.0a12",
    "1.0b1.dev456",
    "1.0b2",
    "1.0b2.post345.dev456",
    "1.0b2.post345",
    "1.0b2-346",
    "1.0c1.dev456",
    "1.0c1",
    "1.0rc2",
    "1.0c3",
    "1.0",
    "1.0.post456.dev34",
    "1.0.post456",
    "1.1.dev1",
    "1.2+123abc",
    "1.2+123abc456",
    "1.2+abc",
    "1.2+abc123",
    "1.2+abc123def",
    "1.2+1234.abc",
    "1.2+123456",
    "1.2.r32+123456",
    "1.2.rev33+123456",
    # Explicit epoch of 1
    "1!1.0.dev456",
    "1!1.0a1",
    "1!1.0a2.dev456",
    "1!1.0a12.dev456",
    "1!1.0a12",
    "1!1.0b1.dev456",
    "1!1.0b2",
    "1!1.0b2.post345.dev456",
    "1!1.0b2.post345",
    "1!1.0b2-346",
    "1!1.0c1.dev456",
    "1!1.0c1",
    "1!1.0rc2",
    "1!1.0c3",
    "1!1.0",
    "1!1.0.post456.dev34",
    "1!1.0.post456",
    "1!1.1.dev1",
    "1!1.2+123abc",
    "1!1.2+123abc456",
    "1!1.2+abc",
    "1!1.2+abc123",
    "1!1.2+abc123def",
    "1!1.2+1234.abc",
    "1!1.2+123456",
    "1!1.2.r32+123456",
    "1!1.2.rev33+123456",
]

# Below we'll generate every possible combination of VERSIONS that
# should be True for the given operator
cases = itertools.chain(
    *
    # Verify that the less than (<) operator works correctly
    [
        ["{} < {}".format(x, y) for y in VERSIONS[i + 1 :]]
        for i, x in enumerate(VERSIONS)
    ]
    +
    # Verify that the less than equal (<=) operator works correctly
    [
        ["{} <= {}".format(x, y) for y in VERSIONS[i:]]
        for i, x in enumerate(VERSIONS)
    ]
    +
    # Verify that the equal (==) operator works correctly
    [["{} == {}".format(x, x) for x in VERSIONS]]
    +
    # Verify that the not equal (!=) operator works correctly
    [
        ["{} != {}".format(x, y) for j, y in enumerate(VERSIONS) if i != j]
        for i, x in enumerate(VERSIONS)
    ]
    +
    # Verify that the greater than equal (>=) operator works correctly
    [
        ["{} >= {}".format(x, y) for y in VERSIONS[: i + 1]]
        for i, x in enumerate(VERSIONS)
    ]
    +
    # Verify that the greater than (>) operator works correctly
    [
        ["{} > {}".format(x, y) for y in VERSIONS[:i]]
        for i, x in enumerate(VERSIONS)
    ]
)

if __name__ == '__main__':
    for case in cases:
        print(case)
