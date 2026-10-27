// Reference only, never imported. Figma node 11:441, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/07f55b2b-0e63-4771-93eb-3bb4d35ff03e";
const imgWorthYes = `${assetPathPrefix}/18c9f.svg`;
const imgWorthOptional = `${assetPathPrefix}/ff725.svg`;
const imgWorthNo = `${assetPathPrefix}/417d2.svg`;
const imgStatusAvailable = `${assetPathPrefix}/9cbc8.svg`;
const imgStatusInProgress = `${assetPathPrefix}/817dd.svg`;
const imgStatusUnknown = `${assetPathPrefix}/99de4.svg`;
const imgManualTick = `${assetPathPrefix}/b7bfe.svg`;
const imgIcon = `${assetPathPrefix}/eef2b.svg`;
const imgChamferBg = `${assetPathPrefix}/7704e.svg`;
const imgChamferBg1 = `${assetPathPrefix}/1db1a.svg`;
const imgChamferStroke = `${assetPathPrefix}/1952a.svg`;

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

type StatusMarkProps = {
  className?: string;
  status?: "available" | "in-progress" | "unknown";
};

function StatusMark({ className, status = "available" }: StatusMarkProps) {
  const isInProgress = status === "in-progress";
  const isUnknown = status === "unknown";
  return (
    <div className={className || "relative size-[16px]"} id={isUnknown ? "node-5_27" : isInProgress ? "node-5_13" : "node-5_8"}>
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={isUnknown ? imgStatusUnknown : isInProgress ? imgStatusInProgress : imgStatusAvailable} />
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
        <StatusMark className="absolute left-0 size-[16px] top-[2px]" status={isInProgressAndManual ? "in-progress" : undefined} />
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

type PanelHeaderProps = {
  className?: string;
  mode?: "play";
};

function PanelHeader({ className, mode = "play" }: PanelHeaderProps) {
  return (
    <div className={className || "bg-[var(--qt-panel-bg,#121416)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[36px] items-center px-[12px] relative w-[320px]"} data-node-id="7:193">
      <div className="relative shrink-0 size-[16px]" data-node-id="7:194" data-name="icon">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
      </div>
      <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[20px] not-italic relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="7:198">
        Elsbeth
      </p>
      <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic overflow-hidden relative text-[12px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis tracking-[0.024px] whitespace-nowrap" data-node-id="7:199">
        Witch 34
      </p>
      <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-secondary,#bdb6a8)] tracking-[0.024px] whitespace-nowrap" data-node-id="7:200">
        Next up
      </p>
    </div>
  );
}

export default function PanelPlay() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="11:441" data-name="Panel (play)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2575" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.12%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-panel-bg,#121416)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_409px] px-[12px] relative shrink-0 w-full" />
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_409px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="11:450" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="11:451">
          Worth it
        </p>
      </div>
      <QuestRow className="bg-[var(--row\/active-bg,#161c24)] content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-64.5px] mask-size-[321px_409px] pb-[10px] pt-[8px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" detection="manual" status="in-progress" />
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-120.5px] mask-size-[321px_409px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:481" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:481;7:268" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:481;7:269" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:481;7:270" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:481;7:271">
              The Lost Tower
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:481;20:868">
          Passive · Cold res
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:481;7:288" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-160.5px] mask-size-[321px_409px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:508" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:508;7:268" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:508;7:269" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:508;7:270" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:508;7:271">
              Keth
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:508;20:868">
          Weapon-set passive
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:508;7:288" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-200.5px] mask-size-[321px_409px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="11:535" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="11:536">
          Optional
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-228.5px] mask-size-[321px_409px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:537" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:537;7:327" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:537;7:328" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:537;7:329" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:537;7:330">
              Desert Crossroads
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:537;20:870">
          Fire res
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:537;7:348" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-268.5px] mask-size-[321px_409px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:565" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:565;7:714" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="unknown" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:565;7:715" data-name="body">
          <div className="content-stretch flex gap-[6px] items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:565;7:716" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I11:565;7:717">
              The Dreadnought
            </p>
            <ManualTick className="relative shrink-0 size-[12px]" />
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:565;20:883">
          Life · Waypoint
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:565;7:745" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-308.5px] mask-size-[321px_409px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="11:604" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="11:605">
          Not worth it
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-336.5px] mask-size-[321px_409px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="11:606" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:606;7:388" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I11:606;7:389" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I11:606;7:390" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis whitespace-nowrap" data-node-id="I11:606;7:391">
              Shrine of the Sands
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I11:606;20:872">
          Item
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I11:606;7:408" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="no" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-376.5px] mask-size-[321px_409px] overflow-clip pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="11:649" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="footer">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] whitespace-nowrap" data-node-id="11:650">
          Act 2 · 1 locked, 1 done, 1 skipped hidden
        </p>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2576" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
