"""Turkey's inputs for any planned row, through tier0's hooks.

- **Population**: the 2022 address register by ilçe, with Syrians under temporary protection, from
  `admin_totals` config TR (`tools/country/turkey_census.py`).
- **Work from home**: Eurostat LFS lfsa_ehomp, Turkey 2025, employed 15-64: 4.1% usually work from
  home, 4.2% sometimes, judged at about 0.6 and 0.2 of their days, as France and Germany.
- **Airports**: DHMI's year-end passenger totals in `airport_passengers.csv`
  (`tools/country/turkey_airports.py`), read by `enrich_airports` the same way as CAAC's bulletin.

Jobs stay the tier 0 proxy. TÜİK's portals have no table API, and Eurostat's NUTS employment
tables have no Turkey rows.
"""
# Eurostat LFS lfsa_ehomp, Turkey 2025, employed 15-64, both sexes: 4.1% usually, 4.2% sometimes
WFH = 0.033


def plan_hooks(row):
    """Population by ilçe and the national work-from-home share for any planned Turkish row."""
    import admin_totals
    return {"admin_totals": admin_totals.hook("TR"), "wfh": WFH}
