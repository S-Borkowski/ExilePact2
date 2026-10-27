// Reference only, never imported. Figma node 11:2, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/c017e1da-2790-479b-b3b4-d8e148e2524d";
const imgStatusLocked = `${assetPathPrefix}/6cf01.svg`;
const imgStatusAvailable = `${assetPathPrefix}/9cbc8.svg`;
const imgStatusInProgress = `${assetPathPrefix}/817dd.svg`;
const imgStatusDone = `${assetPathPrefix}/68736.svg`;
const imgStatusSkipped = `${assetPathPrefix}/e0f10.svg`;
const imgStatusUnknown = `${assetPathPrefix}/99de4.svg`;
const imgWorthYes = `${assetPathPrefix}/18c9f.svg`;
const imgWorthOptional = `${assetPathPrefix}/ff725.svg`;
const imgWorthNo = `${assetPathPrefix}/417d2.svg`;
const imgManualTick = `${assetPathPrefix}/b7bfe.svg`;
const imgChamferBg = `${assetPathPrefix}/12e01.svg`;
const imgChamferBg1 = `${assetPathPrefix}/ff14b.svg`;
const imgIcon = `${assetPathPrefix}/eef2b.svg`;
const imgIcon1 = `${assetPathPrefix}/db69a.svg`;
const imgIcon2 = `${assetPathPrefix}/9d225.svg`;
const imgWorth = `${assetPathPrefix}/eb9e2.svg`;
const imgWorth1 = `${assetPathPrefix}/38ebe.svg`;
const imgChamferStroke = `${assetPathPrefix}/027bb.svg`;

type StatusMarkProps = {
  className?: string;
  status?: "locked" | "available" | "in-progress" | "done" | "skipped" | "unknown";
};

function StatusMark({ className, status = "locked" }: StatusMarkProps) {
  const isAvailable = status === "available";
  const isDone = status === "done";
  const isInProgress = status === "in-progress";
  const isSkipped = status === "skipped";
  const isUnknown = status === "unknown";
  return (
    <div className={className || "relative size-[16px]"} id={isUnknown ? "node-5_27" : isSkipped ? "node-5_23" : isDone ? "node-5_18" : isInProgress ? "node-5_13" : isAvailable ? "node-5_8" : "node-5_2"}>
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={isUnknown ? imgStatusUnknown : isSkipped ? imgStatusSkipped : isDone ? imgStatusDone : isInProgress ? imgStatusInProgress : isAvailable ? imgStatusAvailable : imgStatusLocked} />
    </div>
  );
}

type WorthMarkerProps = {
  className?: string;
  worth?: "yes" | "optional" | "no";
};

function WorthMarker({ className, worth = "yes" }: WorthMarkerProps) {
  const isNo = worth === "no";
  const isOptional = worth === "optional";
  return (
    <div className={className || "relative size-[16px]"} id={isNo ? "node-5_43" : isOptional ? "node-5_38" : "node-5_34"}>
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={isNo ? imgWorthNo : isOptional ? imgWorthOptional : imgWorthYes} />
    </div>
  );
}

function ManualTick({ className }: { className?: string }) {
  return (
    <div className={className || "relative size-[12px]"} data-node-id="5:48" data-name="ManualTick">
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgManualTick} />
    </div>
  );
}

type QuestRowProps = {
  className?: string;
  detection?: "auto" | "manual";
  status?: "available" | "in-progress";
  worth?: "yes";
};

function QuestRow({ className, detection = "auto", status = "available", worth = "yes" }: QuestRowProps) {
  const isInProgressAndManual = status === "in-progress" && detection === "manual";
  return (
    <div className={className || `content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] relative w-[320px] ${isInProgressAndManual ? String.raw`bg-[var(--row\/active-bg,#161c24)] pb-[10px] pt-[8px]` : "py-[10px]"}`} id={isInProgressAndManual ? "node-7_465" : "node-7_262"}>
      <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" id={isInProgressAndManual ? "node-7_471" : "node-7_268"} data-name="status-slot">
        <StatusMark className="absolute left-0 size-[16px] top-[2px]" status={isInProgressAndManual ? "in-progress" : "available"} />
      </div>
      <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" id={isInProgressAndManual ? "node-7_472" : "node-7_269"} data-name="body">
        <div className={`content-stretch flex items-center overflow-clip relative shrink-0 w-full ${isInProgressAndManual ? "gap-[6px]" : ""}`} id={isInProgressAndManual ? "node-7_473" : "node-7_270"} data-name="title">
          <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" id={isInProgressAndManual ? "node-7_474" : "node-7_271"}>
            {isInProgressAndManual ? "A Sunless Sky" : "The Lost Tower"}
          </p>
          {isInProgressAndManual && <ManualTick className="relative shrink-0 size-[12px]" />}
        </div>
        {isInProgressAndManual && (
          <p className="[word-break:break-word] font-['Alegreya_Sans:Regular'] leading-[18px] not-italic overflow-hidden relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis w-full whitespace-nowrap" data-node-id="7:480">
            Find the Sun Key in the Bone Pits
          </p>
        )}
      </div>
      <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" id={isInProgressAndManual ? "node-20_875" : "node-20_868"}>
        Spirit
      </p>
      <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" id={isInProgressAndManual ? "node-7_493" : "node-7_288"} data-name="worth-slot">
        <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
      </div>
      {isInProgressAndManual && <div className="absolute bg-[var(--qt-status-in-progress,#6cb6ff)] bottom-0 left-[3px] top-0 w-[3px]" data-node-id="26:2285" data-name="progress-bar" />}
    </div>
  );
}

export default function PanelPlay() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="11:2" data-name="Panel (play)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2569" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.12%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <div className="bg-[var(--qt-panel-bg,#121416)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_425px] px-[12px] relative shrink-0 w-full" data-node-id="11:3" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="PanelHeader">
        <div className="relative shrink-0 size-[16px]" data-node-id="I11:3;7:194" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[20px] not-italic relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="I11:3;7:198">
          Elsbeth
        </p>
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic overflow-hidden relative text-[12px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis tracking-[0.024px] whitespace-nowrap" data-node-id="I11:3;7:199">
          Witch 34
        </p>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-secondary,#bdb6a8)] tracking-[0.024px] whitespace-nowrap" data-node-id="I11:3;7:200">
          All · Act 2
        </p>
      </div>
      <div className="content-stretch flex flex-col gap-[var(--qt-section-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_425px] overflow-clip relative shrink-0 w-full" data-node-id="11:14" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="content">
        <div className="content-stretch flex flex-col items-start overflow-clip relative shrink-0 w-full" data-node-id="11:24" data-name="section">
          <div className="bg-[var(--qt-panel-raised,#1a1d20)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center pl-[8px] pr-[12px] relative shrink-0 w-full" data-node-id="11:25" data-name="ActHeader">
            <div className="relative shrink-0 size-[16px]" data-node-id="I11:25;7:226" data-name="icon">
              <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
            </div>
            <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans_SC:Bold'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] tracking-[0.39px]" data-node-id="I11:25;7:229">
              Act 2
            </p>
            <div className="content-stretch flex gap-[4px] items-center overflow-clip relative shrink-0" data-node-id="I11:25;7:230" data-name="available">
              <div className="relative shrink-0 size-[12px]" data-node-id="I11:25;7:231" data-name="icon">
                <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
              </div>
              <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-status-available,#7bd88f)] tracking-[0.024px] whitespace-nowrap" data-node-id="I11:25;7:235">
                4 available ·
              </p>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] whitespace-nowrap" data-node-id="I11:25;7:236">
              1/9 done
            </p>
          </div>
          <QuestRow className="bg-[var(--row\/active-bg,#161c24)] content-stretch flex gap-[var(--qt-row-gap,8px)] items-start pb-[10px] pt-[8px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" detection="manual" status="in-progress" />
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:101" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:101;7:268" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="available" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:101;7:269" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:101;7:270" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:101;7:271">
                  The Lost Tower
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:101;20:868">
              Passive · Cold res
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:101;7:288" data-name="worth-slot">
              <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="28:2387" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I28:2387;7:268" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="available" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I28:2387;7:269" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I28:2387;7:270" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I28:2387;7:271">
                  Keth
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I28:2387;20:868">
              Weapon-set passive
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I28:2387;7:288" data-name="worth-slot">
              <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:154" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:154;7:327" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="available" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:154;7:328" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:154;7:329" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:154;7:330">
                  Desert Crossroads
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:154;20:870">
              Fire res
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:154;7:348" data-name="worth-slot">
              <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="28:2403" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I28:2403;7:388" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="available" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I28:2403;7:389" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I28:2403;7:390" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis whitespace-nowrap" data-node-id="I28:2403;7:391">
                  Shrine of the Sands
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I28:2403;20:872">
              Item
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I28:2403;7:408" data-name="worth-slot">
              <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="no" />
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:202" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:202;7:714" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="unknown" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:202;7:715" data-name="body">
              <div className="content-stretch flex gap-[6px] items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:202;7:716" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:202;7:717">
                  The Dreadnought
                </p>
                <ManualTick className="relative shrink-0 size-[12px]" />
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:202;20:883">
              Life · Waypoint
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:202;7:745" data-name="worth-slot">
              <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[6px] relative shrink-0 w-full" data-node-id="11:281" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:281;7:924" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:281;7:925" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:281;7:926" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[14px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis whitespace-nowrap" data-node-id="I11:281;7:927">
                  The Trial of Chaos
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] whitespace-nowrap" data-node-id="I11:281;20:890">
              Ascendancy
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:281;7:938" data-name="worth-slot">
              <div className="absolute left-0 size-[16px] top-[2px]" data-node-id="I11:281;7:934" data-name="worth">
                <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgWorth} />
              </div>
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[6px] relative shrink-0 w-full" data-node-id="11:324" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:324;7:972" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="done" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:324;7:973" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:324;7:974" data-name="title">
                <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[14px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis whitespace-nowrap" data-node-id="I11:324;7:975">
                  Ancient Vows
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] whitespace-nowrap" data-node-id="I11:324;20:892">
              Skill gem
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:324;7:993" data-name="worth-slot">
              <div className="absolute left-0 size-[16px] top-[2px]" data-node-id="I11:324;7:989" data-name="worth">
                <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgWorth1} />
              </div>
            </div>
          </div>
          <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start px-[var(--qt-row-padding-x,12px)] py-[6px] relative shrink-0 w-full" data-node-id="11:380" data-name="QuestRow">
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:380;7:1256" data-name="status-slot">
              <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="skipped" />
            </div>
            <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:380;7:1257" data-name="body">
              <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:380;7:1258" data-name="title">
                <p className="[text-decoration-skip-ink:none] [text-underline-position:from-font] [word-break:break-word] decoration-from-font decoration-solid font-['Alegreya_Sans:Medium'] leading-[20px] line-through max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis whitespace-nowrap" data-node-id="I11:380;7:1259">
                  Treasures of the Tomb
                </p>
              </div>
            </div>
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] whitespace-nowrap" data-node-id="I11:380;20:902">
              Currency
            </p>
            <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:380;7:1272" data-name="worth-slot">
              <div className="absolute left-0 size-[16px] top-[2px]" data-node-id="I11:380;7:1268" data-name="worth">
                <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgWorth} />
              </div>
            </div>
          </div>
        </div>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2570" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
