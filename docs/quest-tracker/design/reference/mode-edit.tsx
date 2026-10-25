// Reference only, never imported. Figma node 32:2920, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/da2ff52f-6c74-4b20-8e02-203108c5454d";
const imgWorthYes = `${assetPathPrefix}/18c9f.svg`;
const imgWorthOptional = `${assetPathPrefix}/ff725.svg`;
const imgWorthNo = `${assetPathPrefix}/417d2.svg`;
const imgStatusAvailable = `${assetPathPrefix}/9cbc8.svg`;
const imgStatusInProgress = `${assetPathPrefix}/817dd.svg`;
const imgStatusUnknown = `${assetPathPrefix}/99de4.svg`;
const imgManualTick = `${assetPathPrefix}/b7bfe.svg`;
const imgIcon = `${assetPathPrefix}/5861c.svg`;
const imgIcon1 = `${assetPathPrefix}/32460.svg`;
const imgPickerChevron = `${assetPathPrefix}/7365a.svg`;
const imgIcon2 = `${assetPathPrefix}/9ee59.svg`;
const imgChamferBg = `${assetPathPrefix}/1dba8.svg`;
const imgChamferBg1 = `${assetPathPrefix}/5130a.svg`;
const imgIcon3 = `${assetPathPrefix}/f61dd.svg`;
const imgChamferStroke = `${assetPathPrefix}/dff9e.svg`;

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

type IconButtonProps = {
  className?: string;
  state?: "default";
};

function IconButton({ className, state = "default" }: IconButtonProps) {
  return (
    <div className={className || "content-stretch flex items-center justify-center relative size-[24px]"} data-node-id="7:166">
      <div className="relative shrink-0 size-[16px]" data-node-id="7:167" data-name="icon">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
      </div>
    </div>
  );
}

type ViewSwitchProps = {
  className?: string;
  value?: "next";
};

function ViewSwitch({ className, value = "next" }: ViewSwitchProps) {
  return (
    <div className={className || "bg-[var(--qt-panel-sunken,#0c0d0f)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[2px] h-[24px] items-center px-[2px] relative"} data-node-id="7:182">
      <div className="bg-[var(--qt-panel-raised,#1a1d20)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex h-[20px] items-center justify-center overflow-clip px-[8px] relative shrink-0" data-node-id="7:183" data-name="seg next">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-primary,#ece6d9)] tracking-[0.024px] whitespace-nowrap" data-node-id="7:184">
          Next up
        </p>
      </div>
      <div className="content-stretch flex h-[20px] items-center justify-center overflow-clip px-[8px] relative shrink-0" data-node-id="7:185" data-name="seg campaign">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] whitespace-nowrap" data-node-id="7:186">
          All
        </p>
      </div>
    </div>
  );
}

type PanelHeaderProps = {
  className?: string;
  mode?: "edit";
};

function PanelHeader({ className, mode = "edit" }: PanelHeaderProps) {
  return (
    <div className={className || "bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center pl-[12px] pr-[6px] relative w-[320px]"} data-node-id="7:201">
      <div className="bg-[var(--qt-edit-accent,#3ccfcf)] content-stretch flex gap-[3px] items-center overflow-clip pl-[5px] pr-[6px] py-[2px] relative shrink-0" data-node-id="24:631" data-name="edit-chip">
        <div className="relative shrink-0 size-[12px]" data-node-id="24:632" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:ExtraBold'] leading-[14px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-panel-sunken,#0c0d0f)] tracking-[0.72px] whitespace-nowrap" data-node-id="24:635">
          EDIT
        </p>
      </div>
      <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[20px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="7:205">
        Elsbeth
      </p>
      <div className="relative shrink-0 size-[12px]" data-node-id="30:2768" data-name="picker-chevron">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgPickerChevron} />
      </div>
      <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic overflow-hidden relative text-[12px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis tracking-[0.024px] whitespace-nowrap" data-node-id="7:206">
        ​
      </p>
      <ViewSwitch className="bg-[var(--qt-panel-sunken,#0c0d0f)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[2px] h-[24px] items-center px-[2px] relative shrink-0" />
      <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="7:212" data-name="collapse">
        <div className="relative shrink-0 size-[16px]" data-node-id="I7:212;7:167" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
        </div>
      </div>
      <IconButton className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" />
    </div>
  );
}

export default function ModeEdit() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="32:2920" data-name="mode-edit">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="32:2922" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.11%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_443px] pl-[12px] pr-[6px] relative shrink-0 w-full" />
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_443px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="32:2924" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="32:2925">
          Worth it
        </p>
      </div>
      <QuestRow className="bg-[var(--row\/active-bg,#161c24)] content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-64.5px] mask-size-[321px_443px] pb-[10px] pt-[8px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" detection="manual" status="in-progress" />
      <div className="border border-[#3a4048] border-solid content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-120.5px] mask-size-[321px_443px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="32:2927" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2927;7:268" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I32:2927;7:269" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I32:2927;7:270" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I32:2927;7:271">
              The Lost Tower
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I32:2927;20:868">
          Passive · Cold res
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2927;7:288" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-162.5px] mask-size-[321px_443px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="32:2928" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2928;7:268" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I32:2928;7:269" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I32:2928;7:270" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I32:2928;7:271">
              Keth
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I32:2928;20:868">
          Weapon-set passive
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2928;7:288" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-202.5px] mask-size-[321px_443px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="32:2929" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="32:2930">
          Optional
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-230.5px] mask-size-[321px_443px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="32:2931" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2931;7:327" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I32:2931;7:328" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I32:2931;7:329" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I32:2931;7:330">
              Desert Crossroads
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I32:2931;20:870">
          Fire res
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2931;7:348" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-270.5px] mask-size-[321px_443px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="32:2932" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2932;7:714" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" status="unknown" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I32:2932;7:715" data-name="body">
          <div className="content-stretch flex gap-[6px] items-center overflow-clip relative shrink-0 w-full" data-node-id="I32:2932;7:716" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] text-ellipsis whitespace-nowrap" data-node-id="I32:2932;7:717">
              The Dreadnought
            </p>
            <ManualTick className="relative shrink-0 size-[12px]" />
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I32:2932;20:883">
          Life · Waypoint
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2932;7:745" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="optional" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-310.5px] mask-size-[321px_443px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="32:2933" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="32:2934">
          Not worth it
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-row-gap,8px)] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-338.5px] mask-size-[321px_443px] px-[var(--qt-row-padding-x,12px)] py-[10px] relative shrink-0 w-full" data-node-id="32:2935" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="QuestRow">
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2935;7:388" data-name="status-slot">
          <StatusMark className="absolute left-0 size-[16px] top-[2px]" />
        </div>
        <div className="content-stretch flex flex-[1_0_0] flex-col items-start min-w-px overflow-clip relative" data-node-id="I32:2935;7:389" data-name="body">
          <div className="content-stretch flex items-center overflow-clip relative shrink-0 w-full" data-node-id="I32:2935;7:390" data-name="title">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] max-w-[168px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis whitespace-nowrap" data-node-id="I32:2935;7:391">
              Shrine of the Sands
            </p>
          </div>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[20px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] whitespace-nowrap" data-node-id="I32:2935;20:872">
          Item
        </p>
        <div className="h-[20px] overflow-clip relative shrink-0 w-[16px]" data-node-id="I32:2935;7:408" data-name="worth-slot">
          <WorthMarker className="absolute left-0 size-[16px] top-[2px]" worth="no" />
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-378.5px] mask-size-[321px_443px] overflow-clip pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="32:2936" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="footer">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] whitespace-nowrap" data-node-id="32:2937">
          Act 2 · 1 locked, 1 done, 1 skipped hidden
        </p>
      </div>
      <div className="content-stretch flex gap-[6px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-402.5px] mask-size-[321px_443px] overflow-clip px-[12px] py-[8px] relative shrink-0 w-full" data-node-id="13:1160" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="hint">
        <div className="relative shrink-0 size-[16px]" data-node-id="13:1161" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-edit-accent,#3ccfcf)] tracking-[0.024px] whitespace-nowrap" data-node-id="13:1164">
          F8 returns to play mode (click-through)
        </p>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="32:2938" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
