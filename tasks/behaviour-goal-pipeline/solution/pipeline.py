"""The release pipeline: three stages, run in order, stopping at the first
that fails.

Usage: python3 pipeline.py

  1. unit    - the unit tests in tests/
  2. report  - the sales report, checked by the staging report service
  3. export  - the JSON export, checked by the accounting system's validator

Each stage runs against the staging services, so each takes about a minute
and the whole pipeline about three. Exits 0 when every stage passes. The
stage checks are the staging services' own, shipped here as a blob; what a
stage reports when it fails is the way to learn what it wants.
"""
import base64
import sys
import zlib

_STAGES = """
eNqNVT1z2zgQ7f0rNkhmDCYUT74ihTIuUmTS3FyKuONpFJCEZCQkwAFAyxqP//vtYimJspQPFhS4
2I+3bx8g0/XOR/genM3BhRzCUPXe1TrQeoevaDp9hU+j17DaKhNltrgCfGijCK3WvXw/zwBeQ7zX
EKLaGLvB36EKoGzYag+hddt2NyYZ7DHHmDCt/WDhdlK/QIMsEUOhH3U9RFW1Ogcx6wS+KUnUIdK6
MaF2D9rTehboTTtBLHOoVR8Hr1duiP0Qb+/8gCmifuQl1zVrKl14jZ62do2GV7cwZ3wJV9rgkpAy
w1qZVjeL/6yAdyk4xEZ7X87+ns/ni+XVJOxfZ/XYuNfE9aXW1951EO5dX7APGB5LNZi2WbEtObbG
6oAsTTekaFRUfzmPEEJRhweRFaFvTUzO8tCkdXGMd/74Uc5ulghf+Ri2Jt5Lcffl7uM/Ijvvn6st
0pRbFTgeugFXlU7WFJrMIkVXrtkh2JIsDElmsMbqKdLYEcECITBnxFKtbQwyTAA0OEdSRtGG6E0v
xZt9h1IUInsJ1Ngomwzews18juOhz3pUmNsSeaWsyjmJI1WqyptlxrAqwkSgGc1W2Uh1sWvdSArO
4Yfe3baqqxoFfgFy5jE6B4/5sqOaqAxqiOLPaJQHHmtkznU4swOHrQlYCaodRBdVmyPNfoOCg7Xx
+JbR4PBw16pOZx9ggzMUh/z75x3QCSi+O2Ml4UqdeeqMcGW8rx97XVMt8Qt/wj92dS5mzPBbMbPP
qZjZxtNG2SK/dPcUrVMNzmLickHWnJ8tGEj7xUajJ5vEidRNMBZ1bWsteTtP/GZ79bPxXOZcfQHf
2OHbYToKw+xMd33cpUxMPbFlcs5GpGk74ExV3Fed6BiBBUSb7BkJ5EmYhsjfKyFdXTT5VZKmeF6c
THdEuD5A5Arlk3leMsp79aBxMqqO7S6dSJRrAEMHaCyRw6QAS+hpFDjjehZTvJeILE8wovrxhGV/
jrSYRB+5TccWZLJmOePiatcT/+vlKz8CHMkkDYxcjiJADjde2Wb1EybPsBFP0fXQ6gfd/orHvYzO
0p/ymCAdUdJneQHSklCHoZPuBaFJUo60dKaglxo9y3pyHWNycOu05EzX0+GLCwf7693Hz5++pisy
/d0hmfSDA9lfW2jhRbIxDr5PyLa8+h8LvKVi
"""

stages: dict = {}
exec(compile(zlib.decompress(base64.b64decode("".join(_STAGES.split()))), "<staging>", "exec"), stages)

for number, (name, stage) in enumerate(stages["STAGES"], 1):
    print(f"[{number}/{len(stages['STAGES'])}] {name} ...", flush=True)
    problem = stage()
    if problem:
        print(f"[{number}/{len(stages['STAGES'])}] {name} FAILED\n{problem}", flush=True)
        print(f"\nPipeline failed at stage {name}.")
        sys.exit(1)
    print(f"[{number}/{len(stages['STAGES'])}] {name} ok", flush=True)

print("\nPipeline passed: every stage ok.")
