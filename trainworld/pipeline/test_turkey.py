"""Turkey's feeds: DHMI passengers in the published-airport table, and the country module's hooks."""
import csv
import unittest
from pathlib import Path

import turkey

HERE = Path(__file__).parent
CSV = HERE / "airport_passengers.csv"
# DHMI All.xlsx attachment 262, PASSENGER sheet, year to date December 2025, arrivals and departures
ISTANBUL = 84_513_937
NATIONAL = 246_776_560


class TurkeyFeeds(unittest.TestCase):
    def test_dhmi_passengers(self):
        with CSV.open(encoding="utf-8", newline="") as f:
            rows = [r for r in csv.DictReader(f) if r["country"] == "TR"]
        by = {r["ident"]: int(r["passengers"]) for r in rows}
        self.assertEqual(by["LTFM"], ISTANBUL)
        self.assertEqual(by["LTFJ"], 48_407_318)
        self.assertEqual(by["LTAI"], 39_000_177)
        self.assertEqual(sum(by.values()), NATIONAL)
        self.assertTrue(all(r["year"] == "2025" and r["iata"] and r["icao"] for r in rows))
        self.assertNotIn("LTAF", by)  # Adana closed in 2025, zero passengers left out

    def test_plan_hooks(self):
        self.assertEqual(turkey.WFH, 0.033)
        try:
            hooks = turkey.plan_hooks({"id": "ist", "iso2": "TR"})
        except ModuleNotFoundError as e:
            self.skipTest(f"pipeline deps not installed ({e})")
        self.assertEqual(set(hooks), {"admin_totals", "wfh"})
        self.assertEqual(hooks["wfh"], 0.033)
        self.assertTrue(callable(hooks["admin_totals"]))


if __name__ == "__main__":
    unittest.main()
