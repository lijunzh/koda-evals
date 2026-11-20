"""The team's lint rule for toolkit/: every public function and method has a
docstring and type hints on each parameter (but `self`) and on its return.

Usage: python3 scripts/lint.py    (exit 0 when clean, 1 with the problems)
"""
import ast
import pathlib
import sys


def problems():
    found = []
    for path in sorted(pathlib.Path("toolkit").glob("*.py")):
        tree = ast.parse(path.read_text(), filename=str(path))
        for node in ast.walk(tree):
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            if node.name.startswith("_"):
                continue
            where = f"{path}:{node.lineno} {node.name}"
            if ast.get_docstring(node) is None:
                found.append(f"{where}: no docstring")
            args = node.args.posonlyargs + node.args.args + node.args.kwonlyargs
            for arg in args:
                if arg.arg in ("self", "cls"):
                    continue
                if arg.annotation is None:
                    found.append(f"{where}: parameter {arg.arg!r} has no type hint")
            if node.returns is None:
                found.append(f"{where}: no return type")
    return found


if __name__ == "__main__":
    found = problems()
    for line in found:
        print(line)
    print(f"{len(found)} problem(s)")
    sys.exit(1 if found else 0)
