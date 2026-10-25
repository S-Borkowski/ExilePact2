// Reference only, never imported. Figma node 12:663, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/90d5581e-edd6-4f9e-a4f8-5a65a4869e53";
const imgChamferBg = `${assetPathPrefix}/d509f.svg`;
const imgChamferBg1 = `${assetPathPrefix}/a4a53.svg`;
const imgChamferBg2 = `${assetPathPrefix}/a166f.svg`;
const imgChamferStroke = `${assetPathPrefix}/32c9d.svg`;
const imgIcon = `${assetPathPrefix}/7ddeb.svg`;
const imgChamferStroke1 = `${assetPathPrefix}/a59db.svg`;
const imgIconRewardResistanceFire = `${assetPathPrefix}/e9095.svg`;
const imgIcon1 = `${assetPathPrefix}/8d76f.svg`;
const imgIcon2 = `${assetPathPrefix}/817dd.svg`;
const imgIcon3 = `${assetPathPrefix}/68736.svg`;
const imgWorthYes = `${assetPathPrefix}/18c9f.svg`;
const imgIcon4 = `${assetPathPrefix}/5861c.svg`;
const imgIcon5 = `${assetPathPrefix}/32460.svg`;
const imgPickerChevron = `${assetPathPrefix}/7365a.svg`;
const imgIcon6 = `${assetPathPrefix}/9ee59.svg`;
const imgChamferBg3 = `${assetPathPrefix}/896e6.svg`;
const imgChamferBg4 = `${assetPathPrefix}/696ef.svg`;
const imgIcon7 = `${assetPathPrefix}/56ea8.svg`;
const imgIconUiManualTick = `${assetPathPrefix}/db497.svg`;
const imgIcon8 = `${assetPathPrefix}/a4330.svg`;
const imgChamferStroke2 = `${assetPathPrefix}/52b3c.svg`;

type ButtonProps = {
  className?: string;
  state?: "default";
  variant?: "primary" | "secondary" | "danger";
};

function Button({ className, state = "default", variant = "primary" }: ButtonProps) {
  const isDanger = variant === "danger";
  const isSecondary = variant === "secondary";
  return (
    <div className={className || "content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center px-[12px] relative w-[80px]"} id={isDanger ? "node-6_164" : isSecondary ? "node-6_156" : "node-6_148"}>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" id={isDanger ? "node-24_605" : isSecondary ? "node-24_593" : "node-24_581"} style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={["secondary", "danger"].includes(variant) ? imgChamferBg2 : imgChamferBg1} />
      </div>
      {["primary", "secondary"].includes(variant) && (
        <p className={`[word-break:break-word] font-["Alegreya_Sans:Bold"] leading-[16px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[80px_28px] not-italic relative shrink-0 text-[13px] tracking-[0.065px] whitespace-nowrap ${isSecondary ? "mask-position-[-28px_-6px] text-[color:var(--qt-text-primary,#ece6d9)]" : "mask-position-[-12px_-6px] text-[color:var(--qt-text-on-light,#121416)]"}`} id={isSecondary ? "node-6_157" : "node-6_149"} style={{ maskImage: `url("${imgChamferBg}")` }}>
          {isSecondary ? "Skip" : "Mark done"}
        </p>
      )}
      {isSecondary && (
        <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:594" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
        </div>
      )}
      {isDanger && (
        <>
          <div className="mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-13px_-6px] mask-size-[80px_28px] relative shrink-0 size-[16px]" data-node-id="6:165" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-37px_-6px] mask-size-[80px_28px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-action-danger,#f28b82)] tracking-[0.065px] whitespace-nowrap" data-node-id="6:168" style={{ maskImage: `url("${imgChamferBg}")` }}>
            Reset
          </p>
          <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:606" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke1} />
          </div>
        </>
      )}
    </div>
  );
}

function IconRewardResistanceFire({ className }: { className?: string }) {
  return (
    <div className={className || "relative size-[24px]"} data-node-id="3:28" data-name="Icon/reward/resistance-fire">
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIconRewardResistanceFire} />
    </div>
  );
}

type RewardChipProps = {
  className?: string;
  label?: "on";
  type?: "spirit" | "resistance-fire";
};

function RewardChip({ className, label = "on", type = "spirit" }: RewardChipProps) {
  const isResistanceFire = type === "resistance-fire";
  const isSpirit = type === "spirit";
  return (
    <div className={className || "bg-[var(--qt-panel-raised,#1a1d20)] content-stretch flex gap-[var(--qt-space-2,8px)] items-center pl-[4px] pr-[8px] py-[2px] relative"} id={isResistanceFire ? "node-5_154" : "node-5_146"}>
      {isSpirit && (
        <>
          <div className="relative shrink-0 size-[20px]" data-node-id="5:147" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="5:153">
            Spirit
          </p>
        </>
      )}
      {isResistanceFire && (
        <>
          <IconRewardResistanceFire className="relative shrink-0 size-[20px]" />
          <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="5:159">
            Fire resistance
          </p>
        </>
      )}
    </div>
  );
}

type StepLineProps = {
  className?: string;
  state?: "pending" | "current" | "done";
};

function StepLine({ className, state = "pending" }: StepLineProps) {
  const isCurrent = state === "current";
  const isCurrentOrDone = ["current", "done"].includes(state);
  const isDone = state === "done";
  const isPending = state === "pending";
  return (
    <div className={className || "content-stretch flex gap-[var(--qt-space-2,8px)] h-[20px] items-center relative w-[288px]"} id={isDone ? "node-6_141" : isCurrent ? "node-6_135" : "node-6_131"}>
      {isCurrentOrDone && (
        <>
          <div className="relative shrink-0 size-[16px]" id={isDone ? "node-6_142" : "node-6_136"} data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={isDone ? imgIcon3 : imgIcon2} />
          </div>
          <p className={`[word-break:break-word] flex-[1_0_0] min-w-px not-italic overflow-hidden relative text-ellipsis whitespace-nowrap ${isDone ? 'font-["Alegreya_Sans:Regular"] leading-[18px] text-[14px] text-[color:var(--qt-text-muted,#959084)]' : 'font-["Alegreya_Sans:Medium"] leading-[16px] text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]'}`} id={isDone ? "node-6_146" : "node-6_140"}>
            {isDone ? "Enter the Vastiri Outskirts" : isCurrent ? "Find the Sun Key in the Bone Pits" : ""}
          </p>
        </>
      )}
      {isPending && (
        <>
          <div className="overflow-clip relative shrink-0 size-[16px]" data-node-id="6:132" data-name="tick">
            <div className="absolute border-[1.5px] border-[var(--qt-text-muted,#959084)] border-solid left-[3px] size-[10px] top-[3px]" data-node-id="6:133" data-name="Rectangle" />
          </div>
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Regular'] leading-[18px] min-w-px not-italic overflow-hidden relative text-[14px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis whitespace-nowrap" data-node-id="6:134">
            Speak to Una at the Ziggurat
          </p>
        </>
      )}
    </div>
  );
}

type WorthMarkerProps = {
  className?: string;
  worth?: "yes";
};

function WorthMarker({ className, worth = "yes" }: WorthMarkerProps) {
  return (
    <div className={className || "relative size-[16px]"} data-node-id="5:34">
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgWorthYes} />
    </div>
  );
}

type StatusMarkProps = {
  className?: string;
  status?: "in-progress";
};

function StatusMark({ className, status = "in-progress" }: StatusMarkProps) {
  return (
    <div className={className || "relative size-[16px]"} data-node-id="5:13">
      <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
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
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon4} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon5} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon6} />
        </div>
      </div>
      <IconButton className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" />
    </div>
  );
}

export default function PanelEdit() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="12:663" data-name="Panel (edit)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2581" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[0_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg4} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_510px] pl-[12px] pr-[6px] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[4px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_510px] overflow-clip p-[8px] relative shrink-0 w-full" data-node-id="12:700" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="back">
        <div className="relative shrink-0 size-[16px]" data-node-id="12:701" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon7} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-secondary,#bdb6a8)] tracking-[0.024px] whitespace-nowrap" data-node-id="12:704">
          Act 2
        </p>
      </div>
      <div className="content-stretch flex flex-col gap-[4px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-68.5px] mask-size-[321px_510px] overflow-clip px-[12px] relative shrink-0 w-full" data-node-id="12:705" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="title">
        <div className="content-stretch flex gap-[8px] items-center overflow-clip relative shrink-0 w-full" data-node-id="12:706" data-name="title line">
          <StatusMark className="relative shrink-0 size-[16px]" />
          <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[24px] not-italic relative shrink-0 text-[20px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="12:712">
            A Sunless Sky
          </p>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-full not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] w-[min-content] whitespace-pre-wrap" data-node-id="12:713">{`Giver: Zarka  ·  Zone: Vastiri Outskirts`}</p>
        <div className="content-stretch flex gap-[6px] items-center overflow-clip relative shrink-0" data-node-id="26:2400" data-name="manual-note">
          <div className="relative shrink-0 size-[12px]" data-node-id="26:2401" data-name="Icon/ui/manual-tick">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIconUiManualTick} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] whitespace-nowrap" data-node-id="26:2404">
            Manual: not tracked by the log; tick steps as you go
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[8px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-132.5px] mask-size-[321px_510px] overflow-clip p-[12px] relative shrink-0 w-full" data-node-id="12:714" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="worth">
        <WorthMarker className="relative shrink-0 size-[16px]" />
        <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col gap-[2px] items-start min-w-px not-italic overflow-clip relative" data-node-id="12:719" data-name="reason">
          <p className="font-['Alegreya_Sans:Medium'] leading-[16px] relative shrink-0 text-[13px] text-[color:var(--qt-worth-gold,#d9b45a)] w-full" data-node-id="12:720">
            Worth it
          </p>
          <p className="font-['Alegreya_Sans:Regular'] leading-[18px] relative shrink-0 text-[14px] text-[color:var(--qt-text-secondary,#bdb6a8)] w-full" data-node-id="12:721">
            Grants Spirit, which you need before your first persistent buff or minion. Short detour from the main route.
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-228.5px] mask-size-[321px_510px] overflow-clip px-[12px] py-[4px] relative shrink-0 w-full" data-node-id="12:722" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="steps heading">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="12:723">
          Steps
        </p>
      </div>
      <div className="content-stretch flex flex-col gap-[8px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-252.5px] mask-size-[321px_510px] overflow-clip px-[16px] py-[4px] relative shrink-0 w-full" data-node-id="12:724" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="steps">
        <div className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[20px] items-center relative shrink-0 w-full" data-node-id="12:725" data-name="StepLine">
          <div className="relative shrink-0 size-[16px]" data-node-id="I12:725;6:142" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
          </div>
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Regular'] leading-[18px] min-w-px not-italic overflow-hidden relative text-[14px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis whitespace-nowrap" data-node-id="I12:725;6:146">
            Speak to Zarka in the Vastiri camp
          </p>
        </div>
        <StepLine className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[20px] items-center relative shrink-0 w-full" state="done" />
        <StepLine className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[20px] items-center relative shrink-0 w-full" state="current" />
        <div className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[20px] items-center relative shrink-0 w-full" data-node-id="12:749" data-name="StepLine">
          <div className="overflow-clip relative shrink-0 size-[16px]" data-node-id="I12:749;6:132" data-name="tick">
            <div className="absolute border-[1.5px] border-[var(--qt-text-muted,#959084)] border-solid left-[3px] size-[10px] top-[3px]" data-node-id="I12:749;6:133" data-name="Rectangle" />
          </div>
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Regular'] leading-[18px] min-w-px not-italic overflow-hidden relative text-[14px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis whitespace-nowrap" data-node-id="I12:749;6:134">
            Open the sealed door
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-364.5px] mask-size-[321px_510px] overflow-clip pb-[4px] pt-[8px] px-[12px] relative shrink-0 w-full" data-node-id="12:753" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="rewards heading">
        <p className="[word-break:break-word] font-['Alegreya_Sans_SC:Bold'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.39px] whitespace-nowrap" data-node-id="12:754">
          Rewards
        </p>
      </div>
      <div className="content-start flex flex-wrap gap-[8px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-392.5px] mask-size-[321px_510px] overflow-clip px-[12px] py-[4px] relative shrink-0 w-full" data-node-id="12:755" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="rewards">
        <RewardChip className="bg-[var(--qt-panel-raised,#1a1d20)] content-stretch flex gap-[var(--qt-space-2,8px)] items-center pl-[4px] pr-[8px] py-[2px] relative shrink-0" />
        <RewardChip className="bg-[var(--qt-panel-raised,#1a1d20)] content-stretch flex gap-[var(--qt-space-2,8px)] items-center pl-[4px] pr-[8px] py-[2px] relative shrink-0" type="resistance-fire" />
      </div>
      <div className="content-stretch flex gap-[6px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-424.5px] mask-size-[321px_510px] overflow-clip px-[12px] py-[8px] relative shrink-0 w-full" data-node-id="12:778" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="poe2db link">
        <p className="[text-underline-position:from-font] [word-break:break-word] decoration-from-font decoration-solid font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] underline whitespace-nowrap" data-node-id="12:779">
          Open on poe2db
        </p>
        <div className="relative shrink-0 size-[16px]" data-node-id="12:780" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon8} />
        </div>
      </div>
      <div className="bg-[var(--qt-panel-divider,#262a2e)] h-px mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-456.5px] mask-size-[321px_510px] relative shrink-0 w-full" data-node-id="12:783" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="divider" />
      <div className="content-stretch flex gap-[8px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-457.5px] mask-size-[321px_510px] overflow-clip pb-[4px] pt-[12px] px-[12px] relative shrink-0 w-full" data-node-id="12:784" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="actions">
        <Button className="content-stretch flex flex-[1_0_0] gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center min-w-px px-[12px] relative" />
        <Button className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center px-[12px] relative shrink-0 w-[80px]" variant="secondary" />
        <Button className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center px-[12px] relative shrink-0 w-[80px]" variant="danger" />
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2582" style={{ maskImage: `url("${imgChamferBg3}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke2} />
      </div>
    </div>
  );
}
