// Reference only, never imported. Figma node 12:881, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/2a9a12f3-b7c4-474d-985f-e667ef1fd325";
const imgIcon = `${assetPathPrefix}/db69a.svg`;
const imgIcon1 = `${assetPathPrefix}/5861c.svg`;
const imgIcon2 = `${assetPathPrefix}/32460.svg`;
const imgPickerChevron = `${assetPathPrefix}/7365a.svg`;
const imgIcon3 = `${assetPathPrefix}/9ee59.svg`;
const imgChamferBg = `${assetPathPrefix}/60b42.svg`;
const imgChamferBg1 = `${assetPathPrefix}/bd142.svg`;
const imgIcon4 = `${assetPathPrefix}/3fd71.svg`;
const imgIcon5 = `${assetPathPrefix}/8234c.svg`;
const imgIcon6 = `${assetPathPrefix}/9f45f.svg`;
const imgChamferStroke = `${assetPathPrefix}/4d491.svg`;

type DropdownProps = {
  className?: string;
  state?: "default";
};

function Dropdown({ className, state = "default" }: DropdownProps) {
  return (
    <div className={className || "bg-[var(--qt-panel-sunken,#0c0d0f)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[var(--qt-space-2,8px)] h-[24px] items-center pl-[8px] pr-[4px] relative w-[140px]"} data-node-id="6:221">
      <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="6:222">
        Auto
      </p>
      <div className="relative shrink-0 size-[16px]" data-node-id="6:223" data-name="icon">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
      </div>
    </div>
  );
}

type SliderProps = {
  className?: string;
  state?: "default";
};

function Slider({ className, state = "default" }: SliderProps) {
  return (
    <div className={className || "h-[24px] relative w-[200px]"} data-node-id="6:204">
      <div className="-translate-y-1/2 absolute bg-[var(--qt-panel-border,#5c636d)] h-[4px] left-0 right-0 top-1/2" data-node-id="6:205" data-name="track" />
      <div className="-translate-y-1/2 absolute bg-[var(--qt-edit-accent,#3ccfcf)] h-[4px] left-0 right-1/4 top-1/2" data-node-id="6:206" data-name="fill" />
      <div className="-translate-y-1/2 absolute bg-[var(--qt-text-primary,#ece6d9)] h-[16px] left-[72.5%] right-[22.5%] top-1/2" data-node-id="6:207" data-name="thumb" />
    </div>
  );
}

type ToggleProps = {
  className?: string;
  state?: "default";
  value?: boolean;
};

function Toggle({ className, state = "default", value = true }: ToggleProps) {
  return (
    <div className={className || "h-[24px] relative w-[36px]"} data-node-id="6:185">
      <div className="absolute bg-[var(--qt-edit-accent,#3ccfcf)] h-[20px] left-0 top-[2px] w-[36px]" data-node-id="6:186" data-name="track" />
      <div className="absolute bg-[var(--qt-text-on-light,#121416)] left-[19px] size-[14px] top-[5px]" data-node-id="6:187" data-name="knob" />
    </div>
  );
}

type SettingRowProps = {
  className?: string;
  control?: "toggle" | "dropdown";
};

function SettingRow({ className, control = "toggle" }: SettingRowProps) {
  const isDropdown = control === "dropdown";
  return (
    <div className={className || "content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center px-[var(--qt-row-padding-x,12px)] relative w-[320px]"} id={isDropdown ? "node-10_957" : "node-10_945"}>
      <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" id={isDropdown ? "node-10_958" : "node-10_946"}>
        {isDropdown ? "Scale" : "Show skipped quests"}
      </p>
      {control === "toggle" && <Toggle className="h-[24px] relative shrink-0 w-[36px]" />}
      {isDropdown && <Dropdown className="bg-[var(--qt-panel-sunken,#0c0d0f)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[var(--qt-space-2,8px)] h-[24px] items-center pl-[8px] pr-[4px] relative shrink-0 w-[140px]" />}
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
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
        </div>
      </div>
      <IconButton className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" />
    </div>
  );
}

export default function PanelEdit() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="12:881" data-name="Panel (edit)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2593" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[0_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_640px] pl-[12px] pr-[6px] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[8px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_640px] overflow-clip p-[12px] relative shrink-0 w-full" data-node-id="12:902" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="title">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Bold'] leading-[20px] min-w-px not-italic relative text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="12:903">
          Settings
        </p>
        <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="12:904" data-name="IconButton">
          <div className="relative shrink-0 size-[16px]" data-node-id="I12:904;7:167" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon4} />
          </div>
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-84.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:911" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="SettingRow">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:911;10:951">
          Background opacity
        </p>
        <Slider className="h-[24px] relative shrink-0 w-[120px]" />
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] h-[16px] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-right w-[32px]" data-node-id="I12:911;10:956">
          92%
        </p>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-124.5px] mask-size-[321px_640px] overflow-clip pb-[6px] px-[12px] relative shrink-0 w-full" data-node-id="28:2509" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="opacity-help">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] w-[296px]" data-node-id="28:2510">
          Text and icons always stay fully opaque. Floor 30%.
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-146.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:921" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="SettingRow">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:921;10:946">
          Click-through in play mode
        </p>
        <Toggle className="h-[24px] relative shrink-0 w-[36px]" />
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-186.5px] mask-size-[321px_640px] overflow-clip pb-[6px] px-[12px] relative shrink-0 w-full" data-node-id="28:2511" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="click-through-help">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] w-[296px]" data-node-id="28:2512">
          Off: the panel blocks clicks to the game.
        </p>
      </div>
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-208.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:928" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="SettingRow">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:928;10:958">
          Dock side
        </p>
        <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border-[length:var(--qt-border-hairline,1px)] border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[var(--qt-space-2,8px)] h-[24px] items-center pl-[8px] pr-[4px] relative shrink-0 w-[140px]" data-node-id="I12:928;10:959" data-name="Dropdown">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:928;10:959;6:222">
            Right
          </p>
          <div className="relative shrink-0 size-[16px]" data-node-id="I12:928;10:959;6:223" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
          </div>
        </div>
      </div>
      <SettingRow className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-248.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" control="dropdown" />
      <SettingRow className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-288.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[40px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-328.5px] mask-size-[321px_640px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:953" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="SettingRow">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:953;10:946">
          Only worth-it quests
        </p>
        <div className="h-[24px] relative shrink-0 w-[36px]" data-node-id="I12:953;10:947" data-name="Toggle">
          <div className="absolute bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid h-[20px] left-0 top-[2px] w-[36px]" data-node-id="I12:953;10:947;6:195" data-name="track" />
          <div className="absolute bg-[var(--qt-text-secondary,#bdb6a8)] left-[3px] size-[14px] top-[5px]" data-node-id="I12:953;10:947;6:196" data-name="knob" />
        </div>
      </div>
      <div className="bg-[var(--qt-panel-divider,#262a2e)] h-px mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-368.5px] mask-size-[321px_640px] relative shrink-0 w-full" data-node-id="30:2994" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="divider" />
      <div className="content-stretch flex flex-col gap-[6px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-369.5px] mask-size-[321px_640px] overflow-clip px-[12px] py-[6px] relative shrink-0 w-full" data-node-id="30:2975" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="hotkeys">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[17px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="30:2976">
          Hotkeys · click a key to rebind
        </p>
        <div className="content-stretch flex gap-[8px] items-center overflow-clip relative shrink-0 w-full" data-node-id="30:2977" data-name="hotkey row">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[17px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)]" data-node-id="30:2978">
            Edit mode
          </p>
          <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex items-start overflow-clip px-[6px] py-px relative shrink-0" data-node-id="30:2979" data-name="keycap">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="30:2980">
              F8
            </p>
          </div>
        </div>
        <div className="content-stretch flex gap-[8px] items-center overflow-clip relative shrink-0 w-full" data-node-id="30:2981" data-name="hotkey row">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[17px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)]" data-node-id="30:2982">
            Collapse / expand
          </p>
          <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex items-start overflow-clip px-[6px] py-px relative shrink-0" data-node-id="30:2983" data-name="keycap">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="30:2984">
              F7
            </p>
          </div>
        </div>
        <div className="content-stretch flex gap-[8px] items-center overflow-clip relative shrink-0 w-full" data-node-id="30:2985" data-name="hotkey row">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[17px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)]" data-node-id="30:2986">
            Switch view
          </p>
          <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex items-start overflow-clip px-[6px] py-px relative shrink-0" data-node-id="30:2987" data-name="keycap">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="30:2988">
              F6
            </p>
          </div>
        </div>
        <div className="content-stretch flex gap-[8px] items-center overflow-clip relative shrink-0 w-full" data-node-id="30:2989" data-name="hotkey row">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[17px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-secondary,#bdb6a8)]" data-node-id="30:2990">
            Icon key
          </p>
          <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex items-start overflow-clip px-[6px] py-px relative shrink-0" data-node-id="30:2991" data-name="keycap">
            <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-primary,#ece6d9)] whitespace-nowrap" data-node-id="30:2992">
              F1
            </p>
          </div>
        </div>
      </div>
      <div className="bg-[var(--qt-panel-divider,#262a2e)] h-px mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-502.5px] mask-size-[321px_640px] relative shrink-0 w-full" data-node-id="12:958" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="divider" />
      <div className="content-stretch flex flex-col gap-[6px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-503.5px] mask-size-[321px_640px] overflow-clip p-[12px] relative shrink-0 w-full" data-node-id="12:959" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="log file">
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] w-full" data-node-id="12:960">
          Log file
        </p>
        <div className="bg-[var(--qt-panel-sunken,#0c0d0f)] border border-[var(--qt-panel-border,#5c636d)] border-solid content-stretch flex gap-[4px] h-[28px] items-center overflow-clip pl-[8px] pr-[2px] relative shrink-0 w-full" data-node-id="12:961" data-name="path">
          <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic overflow-hidden relative text-[12px] text-[color:var(--qt-text-secondary,#bdb6a8)] text-ellipsis tracking-[0.024px] whitespace-nowrap" data-node-id="12:962">
            C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt
          </p>
          <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="12:963" data-name="IconButton">
            <div className="relative shrink-0 size-[16px]" data-node-id="I12:963;7:167" data-name="icon">
              <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon5} />
            </div>
          </div>
        </div>
        <div className="content-stretch flex gap-[6px] items-center overflow-clip relative shrink-0 w-full" data-node-id="12:970" data-name="status">
          <div className="relative shrink-0 size-[16px]" data-node-id="12:971" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon6} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-status-available,#7bd88f)] tracking-[0.024px] whitespace-nowrap" data-node-id="12:975">
            Reading · last line 2 s ago
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-599.5px] mask-size-[321px_640px] overflow-clip pb-[12px] pt-[4px] px-[12px] relative shrink-0 w-full" data-node-id="28:2513" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="icon-key-link">
        <p className="[text-underline-position:from-font] [word-break:break-word] decoration-from-font decoration-solid font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-edit-accent,#3ccfcf)] underline whitespace-nowrap" data-node-id="28:2514">
          Icon key (F1): what each mark means
        </p>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2594" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
