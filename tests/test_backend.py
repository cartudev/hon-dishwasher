import importlib.util
import asyncio
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


BACKEND_PATH = Path(__file__).parents[1] / "src" / "backend.py"
SPEC = importlib.util.spec_from_file_location("dishwasher_backend", BACKEND_PATH)
backend = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(backend)


class Parameter:
    def __init__(self, values, value="0", typology="enum"):
        self.values = values
        self.value = value
        self.typology = typology


class RangeParameter:
    typology = "range"
    min = 0
    max = 2
    step = 1


class Category:
    def __init__(self, parameters):
        self.parameters = parameters


class StartCommand:
    def __init__(self, categories):
        self.categories = categories


class BackendSchemaTests(unittest.TestCase):
    def test_normalizes_localized_program_names(self):
        self.assertEqual(backend.normalize("PRE-LAVADO"), "prelavado")
        self.assertIn(backend.normalize("Prelavaggio"), backend.SETTINGS["programs"]["prewash"]["aliases"])
        self.assertEqual(backend.normalize("Intensivo"), "intensivo")

    def test_checks_enum_and_range_values_without_guessing(self):
        enum = Parameter(["0", "1"])
        fixed = Parameter(["0"], value="0", typology="fixed")
        self.assertTrue(backend.settable(enum, "1"))
        self.assertFalse(backend.settable(fixed, "1"))
        self.assertTrue(backend.allows(RangeParameter(), "2"))
        self.assertFalse(backend.allows(RangeParameter(), "3"))

    def test_intensive_requires_all_three_options_and_detects_baskets(self):
        program = backend.SETTINGS["programs"]["intensive"]
        category = Category({
            "intensive": Parameter(["0", "1"]),
            "extraDry": Parameter(["0", "1"]),
            "openDoor": Parameter(["0", "1"]),
            "diverterLevel": Parameter(["0", "1", "2", "6"]),
            "halfLoad": Parameter(["0", "1"]),
        })
        info = backend.inspect_profile(StartCommand({"intensive": category}), program)
        self.assertTrue(info["available"])
        self.assertEqual(info["basket"]["choices"], ["off", "upper", "lower"])
        self.assertEqual(
            [option["label"] for option in info["required"]],
            ["Intensivo+", "Extra Dry", "Smart Door"],
        )

        del category.parameters["extraDry"]
        info = backend.inspect_profile(StartCommand({"intensive": category}), program)
        self.assertFalse(info["available"])
        self.assertIn("Extra Dry", info["reason"])

    def test_localized_variant_wins_by_declared_alias_order(self):
        # El esquema trae `intensive` y `Intensivo` para el mismo programa: el primer
        # declared in programs.json picks the category.
        program = backend.SETTINGS["programs"]["intensive"]
        english = Category({"intensive": Parameter(["0", "1"]), "extraDry": Parameter(["0", "1"]), "openDoor": Parameter(["0", "1"])})
        spanish = Category({"intensive": Parameter(["0", "1"]), "extraDry": Parameter(["0", "1"]), "openDoor": Parameter(["0", "1"])})
        code, category = backend.get_profile_category(
            StartCommand({"intensive": english, "Intensivo": spanish}), program
        )
        self.assertEqual(code, "intensive")

        # Con el orden invertido en la config, gana la variante localizada.
        reversed_program = dict(program, aliases=["intensivo", "intensive"])
        code, _ = backend.get_profile_category(
            StartCommand({"intensive": english, "Intensivo": spanish}), reversed_program
        )
        self.assertEqual(code, "Intensivo")

    def test_basket_choice_requires_both_half_load_and_diverter(self):
        program = backend.SETTINGS["programs"]["prewash"]
        category = Category({
            "openDoor": Parameter(["0", "1"]),
            "diverterLevel": Parameter(["0", "1", "2"]),
        })
        info = backend.inspect_profile(StartCommand({"prewash": category}), program)
        self.assertFalse(info["basket"]["available"])

    def test_basket_flag_false_never_offers_the_choice(self):
        program = dict(backend.SETTINGS["programs"]["prewash"], basket=False)
        category = Category({
            "openDoor": Parameter(["0", "1"]),
            "diverterLevel": Parameter(["0", "1", "2"]),
            "halfLoad": Parameter(["0", "1"]),
        })
        info = backend.inspect_profile(StartCommand({"prewash": category}), program)
        self.assertFalse(info["basket"]["available"])

    def test_fixed_smart_door_is_supported_only_when_on(self):
        on = Parameter(["1"], value="1", typology="fixed")
        off = Parameter(["0"], value="0", typology="fixed")
        self.assertTrue(backend.parameter_state(on)["supported"])
        self.assertFalse(backend.parameter_state(off)["supported"])

    def test_start_rejects_missing_explicit_confirmation_before_network_access(self):
        with self.assertRaisesRegex(backend.AppError, "Falta confirmación explícita"):
            asyncio.run(backend.dispatch(["start", "intensive", "off", "CANCELAR"]))

    def test_stop_rejects_missing_explicit_confirmation(self):
        with self.assertRaisesRegex(backend.AppError, "Falta confirmación explícita"):
            asyncio.run(backend.dispatch(["stop", "PARAR"]))

    def test_cycle_state_reports_running_and_remaining(self):
        class Appliance:
            def __init__(self, data, attributes):
                self._data = data
                self.attributes = attributes

            def get(self, key, default=None):
                return self._data.get(key, default)

        appliance = Appliance(
            {"machMode": "2", "remainingTimeMM": "74", "prCode": "2"},
            {"active": True, "activity": {"attributes": {"programName": "INTENSIVE"}}},
        )
        state = backend.cycle_state(appliance)
        self.assertTrue(state["running"])
        self.assertEqual(state["remaining_time"], "74")
        self.assertEqual(state["program"], "INTENSIVE")
        idle = Appliance({"machMode": "1", "remainingTimeMM": "0"}, {"active": False, "activity": {}})
        self.assertFalse(backend.cycle_state(idle)["running"])


class ProgramsConfigTests(unittest.TestCase):
    def setUp(self):
        self._original = backend.PROGRAMS_FILE
        self._original_settings = backend.SETTINGS
        self.addCleanup(setattr, backend, "PROGRAMS_FILE", self._original)
        self.addCleanup(setattr, backend, "SETTINGS", self._original_settings)

    def test_user_override_takes_precedence(self):
        with tempfile.TemporaryDirectory() as temporary:
            override = Path(temporary) / "programs.json"
            override.write_text(json.dumps({
                "target": {"applianceType": "WM", "model": "OTRO-123"},
                "programs": [
                    {
                        "id": "cotton",
                        "label": "Algodón",
                        "aliases": ["cotton"],
                        "required": [{"key": "openDoor", "label": "Smart Door"}],
                        "basket": False,
                    }
                ],
            }), encoding="utf-8")
            backend.PROGRAMS_FILE = override
            backend.reload_settings()
            self.assertEqual(backend.SETTINGS["target"]["type"], "WM")
            self.assertIn("cotton", backend.SETTINGS["programs"])
            self.assertEqual(backend.SETTINGS["order"], ["cotton"])
            self.assertFalse(backend.SETTINGS["programs"]["cotton"]["basket"])

    def test_rejects_duplicate_aliases(self):
        with tempfile.TemporaryDirectory() as temporary:
            override = Path(temporary) / "programs.json"
            override.write_text(json.dumps({
                "target": {"applianceType": "DW", "model": "X"},
                "programs": [
                    {"id": "a", "label": "A", "aliases": ["eco"], "required": [], "basket": False},
                    {"id": "b", "label": "B", "aliases": ["ECO "], "required": [], "basket": False},
                ],
            }), encoding="utf-8")
            backend.PROGRAMS_FILE = override
            with self.assertRaisesRegex(backend.AppError, "alias repetido"):
                backend.reload_settings()

    def test_rejects_program_without_required_shape(self):
        with tempfile.TemporaryDirectory() as temporary:
            override = Path(temporary) / "programs.json"
            override.write_text(json.dumps({
                "target": {"applianceType": "DW", "model": "X"},
                "programs": [{"id": "a", "label": "A"}],
            }), encoding="utf-8")
            backend.PROGRAMS_FILE = override
            with self.assertRaisesRegex(backend.AppError, "necesita label y aliases"):
                backend.reload_settings()


class ChoicesTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name) / "hon-dishwasher"
        self._files = (backend.CHOICES_FILE, backend.PROGRAMS_FILE, backend.SETTINGS)
        backend.CHOICES_FILE = root / "choices.json"
        self.addCleanup(setattr, backend, "CHOICES_FILE", self._files[0])
        self.addCleanup(setattr, backend, "PROGRAMS_FILE", self._files[1])
        self.addCleanup(setattr, backend, "SETTINGS", self._files[2])

    def test_options_default_on_and_persist(self):
        self.assertTrue(backend.choice_enabled("intensive", "extraDry"))
        backend.set_option_choice("intensive", "extraDry", False)
        self.assertFalse(backend.choice_enabled("intensive", "extraDry"))
        # Otras opciones y programas quedan en su default.
        self.assertTrue(backend.choice_enabled("intensive", "openDoor"))
        self.assertTrue(backend.choice_enabled("prewash", "openDoor"))
        # An option the program doesn't know is rejected.
        with self.assertRaisesRegex(backend.AppError, "Opción no admitida"):
            backend.set_option_choice("intensive", "noExiste", False)

    def test_disabled_option_that_cannot_be_turned_off_makes_program_unavailable(self):
        program = backend.SETTINGS["programs"]["intensive"]
        category = Category({
            "intensive": Parameter(["0", "1"]),
            "extraDry": Parameter(["1"], value="1", typology="fixed"),
            "openDoor": Parameter(["0", "1"]),
        })
        backend.set_option_choice("intensive", "extraDry", False)
        info = backend.inspect_profile(StartCommand({"intensive": category}), program)
        self.assertFalse(info["available"])
        self.assertIn("Extra Dry", info["reason"])
        backend.set_option_choice("intensive", "extraDry", True)
        info = backend.inspect_profile(StartCommand({"intensive": category}), program)
        self.assertTrue(info["available"])

    def test_program_visibility_roundtrip(self):
        self.assertTrue(backend.program_visible("intensive"))
        backend.set_program_visible("intensive", False)
        self.assertFalse(backend.program_visible("intensive"))
        self.assertTrue(backend.program_visible("prewash"))
        backend.set_program_visible("intensive", True)
        self.assertTrue(backend.program_visible("intensive"))


class RemoteAndCategoryTests(unittest.TestCase):
    def test_remote_ready_is_the_machine_button_not_the_ui_flags(self):
        class Appliance:
            def __init__(self, data):
                self.data = data

            def get(self, key, default=None):
                return self.data.get(key, default)

        ready = Appliance({"remoteCtrValid": "1", "remoteActionable": "0", "remoteVisible": "0"})
        self.assertTrue(backend.remote_control_on(ready))
        ready.data["remoteCtrValid"] = "0"
        self.assertFalse(backend.remote_control_on(ready))

    def test_prewash_is_selected_by_category_not_prcode(self):
        class HonParameterProgram:
            def __init__(self, appliance, categories):
                self._appliance = appliance
                self._categories = categories

            @property
            def value(self):
                return ""

            @value.setter
            def value(self, code):
                self._appliance.commands["startProgram"] = self._categories[code]

        class Command:
            def __init__(self, category):
                self.category = category

        class Appliance:
            def __init__(self):
                self.commands = {}

        appliance = Appliance()
        intensive = Command("intensive")
        prewash = Command("prewash")
        start = Command("intensive")
        start.categories = {"intensive": intensive, "prewash": prewash}
        start.available_settings = {
            "prCode": Parameter(["2", "12"]),
            "category": HonParameterProgram(appliance, start.categories),
        }
        appliance.commands["startProgram"] = intensive
        selected = backend.select_start_category(appliance, start, "prewash")
        self.assertIs(selected, prewash)


class CacheTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        self.cache = root / "last-inspect.json"
        self._original = backend.CACHE_FILE
        backend.CACHE_FILE = self.cache
        self.addCleanup(setattr, backend, "CACHE_FILE", self._original)

    def test_cached_inspect_roundtrips_the_last_query(self):
        state = {"ok": True, "model": "CF 6C4F1PS-12", "profiles": {}}
        backend.save_cache(state)
        info = backend.cached_inspect()
        self.assertTrue(info["ok"])
        self.assertTrue(info["cached"])
        self.assertEqual(info["model"], "CF 6C4F1PS-12")

    def test_cached_inspect_without_a_file_reports_clearly(self):
        with self.assertRaisesRegex(backend.AppError, "no hay una consulta guardada"):
            backend.cached_inspect()


class StoreCredentialsTests(unittest.TestCase):
    def setUp(self):
        self._original_secret_tool = backend.secret_tool
        self._original_stdin = backend.sys.stdin
        self._original_use_secret_tool = backend.USE_SECRET_TOOL
        self.addCleanup(setattr, backend, "USE_SECRET_TOOL", self._original_use_secret_tool)
        backend.USE_SECRET_TOOL = True
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        backend.CONFIG_DIR = root / "hon-dishwasher"
        backend.CONFIG_FILE = backend.CONFIG_DIR / "config.json"
        self.calls = []

        def fake_secret_tool(*args, input_text=None):
            self.calls.append((args, input_text))
            return subprocess.CompletedProcess(args, 0, "", "")

        backend.secret_tool = fake_secret_tool
        self.addCleanup(setattr, backend, "secret_tool", self._original_secret_tool)
        self.addCleanup(setattr, backend.sys, "stdin", self._original_stdin)

    def test_store_saves_email_only_to_disk_and_password_to_keyring(self):
        backend.sys.stdin = io.StringIO(json.dumps({"email": "a@b.com", "password": "secreta"}))
        info = backend.store_credentials()
        self.assertTrue(info["ok"])
        config = json.loads(backend.CONFIG_FILE.read_text(encoding="utf-8"))
        self.assertEqual(config["email"], "a@b.com")
        self.assertNotIn("password", json.dumps(config))
        args, input_text = self.calls[0]
        self.assertIn("store", args)
        self.assertIn("service", args)
        self.assertEqual(input_text, "secreta\n")

    def test_store_rejects_empty_password_without_touching_config(self):
        backend.sys.stdin = io.StringIO(json.dumps({"email": "a@b.com", "password": ""}))
        with self.assertRaisesRegex(backend.AppError, "contraseña está vacía"):
            backend.store_credentials()
        self.assertFalse(backend.CONFIG_FILE.exists())
        self.assertEqual(self.calls, [])

    def test_store_rejects_invalid_payload(self):
        backend.sys.stdin = io.StringIO("no-es-json")
        with self.assertRaisesRegex(backend.AppError, "no es válida"):
            backend.store_credentials()
        self.assertFalse(backend.CONFIG_FILE.exists())


if __name__ == "__main__":
    unittest.main()
