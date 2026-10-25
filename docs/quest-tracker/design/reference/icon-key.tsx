// Reference only, never imported. Figma node 28:2749, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/90bcc161-6253-4dc5-b8b6-3e3649c82f59";
const imgManualTick = `${assetPathPrefix}/b7bfe.svg`;
const imgWorthYes = `${assetPathPrefix}/18c9f.svg`;
const imgWorthOptional = `${assetPathPrefix}/ff725.svg`;
const imgWorthNo = `${assetPathPrefix}/417d2.svg`;
const imgStatusLocked = `${assetPathPrefix}/6cf01.svg`;
const imgStatusAvailable = `${assetPathPrefix}/9cbc8.svg`;
const imgStatusInProgress = `${assetPathPrefix}/817dd.svg`;
const imgStatusDone = `${assetPathPrefix}/68736.svg`;
const imgStatusSkipped = `${assetPathPrefix}/e0f10.svg`;
const imgStatusUnknown = `${assetPathPrefix}/99de4.svg`;
const imgIcon = `${assetPathPrefix}/5861c.svg`;
const imgIcon1 = `${assetPathPrefix}/32460.svg`;
const imgPickerChevron = `${assetPathPrefix}/7365a.svg`;
const imgIcon2 = `${assetPathPrefix}/9ee59.svg`;
const imgChamferBg = `${assetPathPrefix}/dab93.svg`;
const imgChamferBg1 = `${assetPathPrefix}/2d099.svg`;
const imgIcon3 = `${assetPathPrefix}/3fd71.svg`;
const imgChamferStroke = `${assetPathPrefix}/f4edf.svg`;

function ManualTick({ className }: { className?: string }) {
  return (
    <div className={className || "relative size-[12px]"} data-node-id="5:48" data-name="ManualTick">
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgManualTick} />
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

export default function IconKey() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="28:2749" data-name="icon-key">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="28:2751" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[0_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_639px] pl-[12px] pr-[6px] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[8px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_639px] overflow-clip p-[12px] relative shrink-0 w-full" data-node-id="28:2753" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="title">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Bold'] leading-[20px] min-w-px not-italic relative text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2754">
          Icon key
        </p>
        <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="28:2755" data-name="IconButton">
          <div className="relative shrink-0 size-[16px]" data-node-id="I28:2755;7:167" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
          </div>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-84.5px] mask-size-[321px_639px] overflow-clip pb-[4px] px-[12px] relative shrink-0 w-full" data-node-id="30:3015" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="f1-note">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] whitespace-nowrap" data-node-id="30:3016">
          F1 opens this anywhere. Esc closes it.
        </p>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-104.5px] mask-size-[321px_639px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="28:2829" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[normal] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.96px] whitespace-nowrap" data-node-id="28:2830">
          STATUS
        </p>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-130.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2831" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" status="available" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2837" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2838">
            Available
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2839">
            Can be picked up now
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-174.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2840" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" status="in-progress" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2846" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2847">
            In progress
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2848">
            Started; next step shown below the name
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-218.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2849" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" status="done" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2855" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2856">
            Done
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2857">
            Finished
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-262.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2858" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2865" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2866">
            Locked
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2867">
            Not reachable yet
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-306.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2868" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" status="skipped" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2873" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2874">
            Skipped
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2875">
            You chose not to do it
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-350.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2876" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <StatusMark className="relative shrink-0 size-[16px]" status="unknown" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2883" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2884">
            Unknown
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2885">
            Story moved on with no record. Mark it done or skipped.
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-394.5px] mask-size-[321px_639px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="28:2886" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[normal] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.96px] whitespace-nowrap" data-node-id="28:2887">
          WORTH IT
        </p>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-420.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2888" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <WorthMarker className="relative shrink-0 size-[16px]" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2893" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2894">
            Worth it
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2895">
            Permanent reward worth the detour
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-464.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2896" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <WorthMarker className="relative shrink-0 size-[16px]" worth="optional" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2901" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2902">
            Optional
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2903">
            Useful, not essential
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-508.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2904" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <WorthMarker className="relative shrink-0 size-[16px]" worth="no" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2909" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2910">
            Not worth it
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2911">
            Story or minor reward
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-552.5px] mask-size-[321px_639px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="28:2912" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="group label">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[normal] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.96px] whitespace-nowrap" data-node-id="28:2913">
          DETECTION
        </p>
      </div>
      <div className="content-stretch flex gap-[10px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-578.5px] mask-size-[321px_639px] overflow-clip px-[12px] py-[5px] relative shrink-0 w-full" data-node-id="28:2914" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="key row">
        <ManualTick className="relative shrink-0 size-[12px]" />
        <div className="[word-break:break-word] content-stretch flex flex-col font-['Alegreya_Sans:Medium'] items-start not-italic overflow-clip relative shrink-0 whitespace-nowrap" data-node-id="28:2919" data-name="text">
          <p className="leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="28:2920">
            Manual
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)]" data-node-id="28:2921">
            Not tracked by the log; tick steps as you go
          </p>
        </div>
      </div>
      <div className="h-[8px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-622.5px] mask-size-[321px_639px] relative shrink-0 w-[10px]" data-node-id="28:2922" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="bottom-pad" />
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="28:2777" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
