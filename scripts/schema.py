"""Dumps the startProgram schema (read-only) to tune config/programs.json.

Uso: .venv/bin/python scripts/schema.py [comando]
With no argument it lists the commands and, for startProgram, its categories and parameters.
"""

import asyncio
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
import backend  # noqa: E402


async def main() -> None:
    hon, appliance = await backend.connect_device()
    try:
        if len(sys.argv) > 1 and sys.argv[1] == "attributes":
            parameters = (appliance.attributes or {}).get("parameters", {})
            print("activity:", repr(appliance.attributes.get("activity")))
            print("active:", repr(appliance.attributes.get("active")))
            for key in sorted(parameters):
                value = parameters[key]
                print(f"{key} = {getattr(value, 'value', value)!r}")
            return
        start = appliance.commands.get("startProgram")
        if start is None:
            print("no hay startProgram")
            return
        print(f"comando: startProgram")
        print(f"available_settings: {sorted(start.available_settings)}")
        for code, category in (getattr(start, "categories", {}) or {}).items():
            print(f"\n== category {code!r} ({backend.normalize(code)!r})")
            params = getattr(category, "parameters", {}) or {}
            for key, parameter in params.items():
                typology = str(getattr(parameter, "typology", ""))
                extra = ""
                if typology == "enum":
                    extra = f" valores={list(parameter.values)}"
                elif typology == "range":
                    extra = f" min={parameter.min} max={parameter.max} step={parameter.step}"
                elif typology == "fixed":
                    extra = f" value={getattr(parameter, 'value', None)}"
                print(f"   {key} [{typology}]{extra}")
    finally:
        await hon.__aexit__(None, None, None)


if __name__ == "__main__":
    asyncio.run(main())
