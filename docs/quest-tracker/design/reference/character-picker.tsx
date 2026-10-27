// Reference only, never imported. Figma node 12:796, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/7b5f5062-4120-41a4-86e7-6d0ca76c5eca";
const imgIcon = `${assetPathPrefix}/b725f.svg`;
const imgIcon1 = `${assetPathPrefix}/0082a.svg`;
const imgIcon2 = `${assetPathPrefix}/5861c.svg`;
const imgIcon3 = `${assetPathPrefix}/32460.svg`;
const imgPickerChevron = `${assetPathPrefix}/7365a.svg`;
const imgIcon4 = `${assetPathPrefix}/9ee59.svg`;
const imgChamferBg = `${assetPathPrefix}/88295.svg`;
const imgChamferBg1 = `${assetPathPrefix}/234a9.svg`;
const imgIcon5 = `${assetPathPrefix}/3fd71.svg`;
const imgIcon6 = `${assetPathPrefix}/03eae.svg`;
const imgChamferBg2 = `${assetPathPrefix}/e9f82.svg`;
const imgChamferBg3 = `${assetPathPrefix}/05683.svg`;
const imgChamferStroke = `${assetPathPrefix}/61169.svg`;
const imgChamferStroke1 = `${assetPathPrefix}/cac2e.svg`;

type CharacterRowProps = {
  className?: string;
  state?: "active";
};

function CharacterRow({ className, state = "active" }: CharacterRowProps) {
  return (
    <div className={className || "bg-[var(--select\\/active-bg,#142628)] border-[var(--qt-edit-accent,#3ccfcf)] border-l-2 border-solid content-stretch flex gap-[var(--qt-space-3,12px)] h-[48px] items-center px-[var(--qt-row-padding-x,12px)] relative w-[320px]"} data-node-id="10:914">
      <div className="relative shrink-0 size-[20px]" data-node-id="10:915" data-name="icon">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
      </div>
      <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col font-['Alegreya_Sans:Medium'] items-start min-w-px not-italic overflow-clip relative whitespace-nowrap" data-node-id="10:919" data-name="text">
        <p className="leading-[20px] relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="10:920">
          Elsbeth
        </p>
        <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px]" data-node-id="10:921">
          Witch · Level 34
        </p>
      </div>
      <div className="content-stretch flex gap-[4px] items-center overflow-clip relative shrink-0" data-node-id="10:922" data-name="active">
        <div className="relative shrink-0 size-[16px]" data-node-id="10:923" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-edit-accent,#3ccfcf)] tracking-[0.024px] whitespace-nowrap" data-node-id="10:927">
          Tracking
        </p>
      </div>
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
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
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
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon4} />
        </div>
      </div>
      <IconButton className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" />
    </div>
  );
}

export default function PanelEdit() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="12:796" data-name="Panel (edit)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2587" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.14%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_353px] pl-[12px] pr-[6px] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[8px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_353px] overflow-clip p-[12px] relative shrink-0 w-full" data-node-id="12:817" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="title">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Bold'] leading-[20px] min-w-px not-italic relative text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="12:818">
          Characters
        </p>
        <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="12:819" data-name="IconButton">
          <div className="relative shrink-0 size-[16px]" data-node-id="I12:819;7:167" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon5} />
          </div>
        </div>
      </div>
      <CharacterRow className="bg-[var(--select\/active-bg,#142628)] border-[var(--qt-edit-accent,#3ccfcf)] border-l-2 border-solid content-stretch flex gap-[var(--qt-space-3,12px)] h-[48px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-84.5px] mask-size-[321px_353px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" />
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[48px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-132.5px] mask-size-[321px_353px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:846" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="CharacterRow">
        <div className="relative shrink-0 size-[20px]" data-node-id="I12:846;10:929" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon6} />
        </div>
        <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col font-['Alegreya_Sans:Medium'] items-start min-w-px not-italic overflow-clip relative whitespace-nowrap" data-node-id="I12:846;10:933" data-name="text">
          <p className="leading-[20px] relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:846;10:934">
            Brann
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px]" data-node-id="I12:846;10:935">
            Warrior · Level 21
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[48px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-180.5px] mask-size-[321px_353px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:857" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="CharacterRow">
        <div className="relative shrink-0 size-[20px]" data-node-id="I12:857;10:929" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon6} />
        </div>
        <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col font-['Alegreya_Sans:Medium'] items-start min-w-px not-italic overflow-clip relative whitespace-nowrap" data-node-id="I12:857;10:933" data-name="text">
          <p className="leading-[20px] relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:857;10:934">
            Ilse
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px]" data-node-id="I12:857;10:935">
            Ranger · Level 12
          </p>
        </div>
      </div>
      <div className="content-stretch flex gap-[var(--qt-space-3,12px)] h-[48px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-228.5px] mask-size-[321px_353px] px-[var(--qt-row-padding-x,12px)] relative shrink-0 w-full" data-node-id="12:868" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="CharacterRow">
        <div className="relative shrink-0 size-[20px]" data-node-id="I12:868;10:929" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon6} />
        </div>
        <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col font-['Alegreya_Sans:Medium'] items-start min-w-px not-italic overflow-clip relative whitespace-nowrap" data-node-id="I12:868;10:933" data-name="text">
          <p className="leading-[20px] relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)]" data-node-id="I12:868;10:934">
            Tomas
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px]" data-node-id="I12:868;10:935">
            Mercenary · Level 4
          </p>
        </div>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-276.5px] mask-size-[321px_353px] overflow-clip px-[12px] py-[8px] relative shrink-0 w-full" data-node-id="12:876" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="note">
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans:Medium'] leading-[16px] min-w-px not-italic relative text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px]" data-node-id="12:877">
          Detected from Client.txt when you enter a zone.
        </p>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-308.5px] mask-size-[321px_353px] overflow-clip px-[12px] py-[4px] relative shrink-0 w-full" data-node-id="12:878" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="actions">
        <div className="content-stretch flex flex-[1_0_0] gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center min-w-px px-[12px] relative" data-node-id="12:879" data-name="Button">
          <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="I12:879;24:593" style={{ maskImage: `url("${imgChamferBg2}")` }} data-name="chamfer-bg">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferBg3} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-28px_-6px] mask-size-[80px_28px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] tracking-[0.065px] whitespace-nowrap" data-node-id="I12:879;6:157" style={{ maskImage: `url("${imgChamferBg2}")` }}>
            New character
          </p>
          <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="I12:879;24:594" style={{ maskImage: `url("${imgChamferBg2}")` }} data-name="chamfer-stroke">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
          </div>
        </div>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2588" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke1} />
      </div>
    </div>
  );
}
