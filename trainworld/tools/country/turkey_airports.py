"""DHMI's year-end passengers for pipeline/airport_passengers.csv, the same slot as CAAC's bulletin.

    python tools/country/turkey_airports.py

Downloads DHMI's English All.xlsx for year-to-date December 2025 (attachment 262 on the statistics
page) and replaces the TR rows. Passengers are arrivals plus departures. Airports with no 2025
traffic are left out. OurAirports ident, IATA and ICAO are the codes enrich_airports matches on.
"""
import csv
import io
import os
import subprocess
import sys
import tempfile

import openpyxl

TW = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CSV = os.path.join(TW, "pipeline", "airport_passengers.csv")
URL = "https://dhmi.gov.tr/Lists/Istatislikler_EN/Attachments/262/All.xlsx"
YEAR = 2025
SOURCE = ("DHMI passenger traffic, arrivals and departures, year to date December 2025 "
          "(https://dhmi.gov.tr/Lists/Istatislikler_EN/Attachments/262/All.xlsx)")
# DHMI's PASSENGER sheet name -> OurAirports ident, IATA, ICAO
AIRPORTS = {
    "İstanbul (*)": ("LTFM", "IST", "LTFM"),
    "İstanbul Sabiha Gökçen (*)": ("LTFJ", "SAW", "LTFJ"),
    "Ankara Esenboğa": ("LTAC", "ESB", "LTAC"),
    "İzmir Adnan Menderes": ("LTBJ", "ADB", "LTBJ"),
    "Antalya": ("LTAI", "AYT", "LTAI"),
    "Gazipaşa Alanya (*)": ("LTFG", "GZP", "LTFG"),
    "Muğla Dalaman": ("LTBS", "DLM", "LTBS"),
    "Muğla Milas-Bodrum": ("LTFE", "BJV", "LTFE"),
    "Adana": ("LTAF", "ADA", "LTAF"),
    "Çukurova (*)": ("LTDB", "COV", "LTDB"),
    "Trabzon": ("LTCG", "TZX", "LTCG"),
    "Erzurum": ("LTCE", "ERZ", "LTCE"),
    "Gaziantep": ("LTAJ", "GZT", "LTAJ"),
    "Adıyaman": ("LTCP", "ADF", "LTCP"),
    "Ağrı Ahmed-i Hani": ("LTCO", "AJI", "LTCO"),
    "Amasya Merzifon": ("LTAP", "MZH", "LTAP"),
    "Aydın Çıldır (*)": ("LTBD", "CII", "LTBD"),
    "Balıkesir Koca Seyit": ("LTFD", "EDO", "LTFD"),
    "Balıkesir Merkez": ("LTBF", "BZI", "LTBF"),
    "Batman": ("LTCJ", "BAL", "LTCJ"),
    "Bingöl": ("LTCU", "BGG", "LTCU"),
    "Bursa Yenişehir": ("LTBR", "YEI", "LTBR"),
    "Çanakkale": ("LTBH", "CKZ", "LTBH"),
    "Çanakkale Gökçeada": ("LTFK", "GKD", "LTFK"),
    "Denizli Çardak": ("LTAY", "DNZ", "LTAY"),
    "Diyarbakır": ("LTCC", "DIY", "LTCC"),
    "Elazığ": ("LTCA", "EZS", "LTCA"),
    "Erzincan Yıldırım Akbulut": ("LTCD", "ERC", "LTCD"),
    "Eskişehir Hasan Polatkan (*)": ("LTBY", "AOE", "LTBY"),
    "Hakkari Yüksekova Selahaddin Eyyubi": ("LTCW", "YKO", "LTCW"),
    "Hatay": ("LTDA", "HTY", "LTDA"),
    "Iğdır Şehit Bülent Aydın": ("LTCT", "IGD", "LTCT"),
    "Isparta Süleyman Demirel": ("LTFC", "ISE", "LTFC"),
    "Kahramanmaraş": ("LTCN", "KCM", "LTCN"),
    "Kapadokya": ("LTAZ", "NAV", "LTAZ"),
    "Kars Harakani": ("LTCF", "KSY", "LTCF"),
    "Kastamonu": ("LTAL", "KFS", "LTAL"),
    "Kayseri": ("LTAU", "ASR", "LTAU"),
    "Kocaeli Cengiz Topel": ("LTBQ", "KCO", "LTBQ"),
    "Konya": ("LTAN", "KYA", "LTAN"),
    "Malatya": ("LTAT", "MLX", "LTAT"),
    "Mardin Prof. Dr. Aziz Sancar": ("LTCR", "MQM", "LTCR"),
    "Muş Sultan Alparslan": ("LTCK", "MSR", "LTCK"),
    "Ordu-Giresun": ("LTCB", "OGU", "LTCB"),
    "Rize-Artvin": ("LTFO", "RZV", "LTFO"),
    "Samsun Çarşamba": ("LTFH", "SZF", "LTFH"),
    "Siirt": ("LTCL", "SXZ", "LTCL"),
    "Sinop": ("LTCM", "NOP", "LTCM"),
    "Sivas Nuri Demirağ": ("LTAR", "VAS", "LTAR"),
    "Şanlıurfa Gap": ("LTCS", "GNY", "LTCS"),
    "Şırnak Şerafettin Elçi": ("LTCV", "NKT", "LTCV"),
    "Tekirdağ Çorlu Atatürk": ("LTBU", "TEQ", "LTBU"),
    "Tokat": ("LTAW", "TJK", "LTAW"),
    "Uşak": ("LTBO", "USQ", "LTBO"),
    "Van Ferit Melen": ("LTCI", "VAN", "LTCI"),
    "Zafer (*)": ("LTBZ", "KZR", "LTBZ"),
    "Zonguldak Çaycuma (*)": ("LTAS", "ONQ", "LTAS"),
    "İstanbul Atatürk": ("LTBA", "ISL", "LTBA"),
}


def workbook():
    fd, path = tempfile.mkstemp(suffix=".xlsx")
    os.close(fd)
    subprocess.run(["curl.exe", "-sS", "-L", "--fail", "-A", "Mozilla/5.0", "-o", path, URL], check=True)
    return path


def passengers(path):
    """{DHMI name: 2025 arrivals+departures} and the sheet's Turkey total, direct transit left out."""
    wb = openpyxl.load_workbook(path, data_only=True, read_only=True)
    ws = wb["PASSENGER"]
    rows = list(ws.iter_rows(min_row=4, values_only=True))
    wb.close()
    out, total = {}, None
    for row in rows:
        name = row[0]
        if not name or str(name).startswith("(*)"):
            continue
        if name == "TOTAL OF  TURKEY":
            total = int(row[6])
            continue
        if str(name).startswith("TOTAL") or str(name).startswith("DHMI") or str(name).startswith("OTHER"):
            continue
        if name not in AIRPORTS:
            raise SystemExit(f"no OurAirports code for {name!r}")
        out[name] = int(row[6] or 0)
    if total is None or sum(out.values()) != total:
        raise SystemExit(f"airport rows {sum(out.values()):,} against TOTAL OF TURKEY {total}")
    return out


def main():
    path = workbook()
    try:
        counts = passengers(path)
    finally:
        os.remove(path)
    extra = []
    for name, pax in counts.items():
        if pax <= 0:
            continue
        ident, iata, icao = AIRPORTS[name]
        extra.append({"country": "TR", "ident": ident, "iata": iata, "icao": icao, "name": name,
                      "passengers": str(pax), "year": str(YEAR), "source": SOURCE})
    # keep every other country's lines as they are; only the TR block is rewritten
    prior = [ln for ln in open(CSV, encoding="utf-8") if not ln.startswith("TR,")]
    if prior and not prior[-1].endswith("\n"):
        prior[-1] += "\n"
    buf = io.StringIO()
    w = csv.DictWriter(buf, fieldnames=["country", "ident", "iata", "icao", "name", "passengers", "year", "source"],
                       lineterminator="\n")
    w.writerows(extra)
    with open(CSV, "w", encoding="utf-8", newline="") as f:
        f.writelines(prior)
        f.write(buf.getvalue())
    print(f"{CSV}: {len(extra)} Turkish airports, {sum(int(r['passengers']) for r in extra):,} passengers in {YEAR}")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
