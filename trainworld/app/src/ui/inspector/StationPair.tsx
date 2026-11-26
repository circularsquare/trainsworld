// The station view's row for a one-way station (a station on one-way single track) that can be one stop with
// a station on the other way's track (game/pairs.ts): the station it is paired with and Unpair, or Pair, which
// picks the other half on the map. Shown only where there is a pair or one to make.
import { t } from "../../i18n";
import { cancelPair, pairing, startPair, unpair } from "../../game/pairs";
import type { StationInput } from "../../game/types";
import { mobileDevice } from "../../workers/deviceMemory";
import { world } from "../../game/world";
import { useClockQuery } from "../useClockQuery";

export function StationPair({ s }: { s: StationInput }) {
  const answer = useClockQuery({ what: "stationPair", node: s.num }, [s.num], (d) => ({ pair: d[0] ?? -1, options: Array.from(d.slice(1)) }));
  const w = world.value;
  if (!answer || !w || (answer.pair < 0 && answer.options.length === 0)) return null;
  const other = answer.pair >= 0 ? w.save.stations.find((x) => x.num === answer.pair) : undefined;
  const picking = pairing.value === s.id;
  return (
    <>
      <li>
        <span class="grow">{other ? t("stationPair.pairedWith", { name: other.name }) : t("stationPair.oneWay")}</span>
        {other
          ? <button class="btn text" data-tip={t("stationPair.unpairTip")} onClick={() => void unpair(s.id)}>{t("stationPair.unpair")}</button>
          : <button class={"btn text" + (picking ? " on" : "")} aria-pressed={picking} data-tip={t("stationPair.pairTip")}
              onClick={() => (picking ? cancelPair() : startPair(s.id))}>{t("stationPair.pair")}</button>}
      </li>
      {picking && <li class="muted">{t(mobileDevice() ? "stationPair.pickingTouch" : "stationPair.picking")}</li>}
    </>
  );
}
