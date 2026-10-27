// Reference only, never imported. Figma node 13:1266, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/e173b844-38d5-40c1-ab1d-fe1b74b7592d";
const imgIcon = `${assetPathPrefix}/eef2b.svg`;
const imgChamferBg = `${assetPathPrefix}/5153d.svg`;
const imgChamferBg1 = `${assetPathPrefix}/be847.svg`;
const imgIcon1 = `${assetPathPrefix}/db69a.svg`;
const imgIcon2 = `${assetPathPrefix}/91b61.svg`;
const imgChamferStroke = `${assetPathPrefix}/9526f.svg`;

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

export default function EmptyActDone() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="13:1266" data-name="empty-act-done">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2620" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.35%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <PanelHeader className="bg-[var(--qt-panel-bg,#121416)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_143px] px-[12px] relative shrink-0 w-full" />
      <div className="bg-[var(--qt-panel-raised,#1a1d20)] content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_143px] pl-[8px] pr-[12px] relative shrink-0 w-full" data-node-id="13:1275" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="ActHeader">
        <div className="relative shrink-0 size-[16px]" data-node-id="I13:1275;7:238" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
        </div>
        <p className="[word-break:break-word] flex-[1_0_0] font-['Alegreya_Sans_SC:Bold'] leading-[16px] min-w-px not-italic relative text-[13px] text-[color:var(--qt-text-primary,#ece6d9)] tracking-[0.39px]" data-node-id="I13:1275;7:241">
          Act 2
        </p>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Medium'] leading-[16px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-text-muted,#959084)] tracking-[0.024px] whitespace-nowrap" data-node-id="I13:1275;7:242">
          9/9
        </p>
      </div>
      <div className="content-stretch flex gap-[12px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-64.5px] mask-size-[321px_143px] overflow-clip px-[12px] py-[16px] relative shrink-0 w-full" data-node-id="13:1283" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="act complete">
        <div className="relative shrink-0 size-[24px]" data-node-id="13:1284" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
        </div>
        <div className="[word-break:break-word] content-stretch flex flex-[1_0_0] flex-col font-['Alegreya_Sans:Medium'] gap-[2px] items-start min-w-px not-italic overflow-clip relative" data-node-id="13:1288" data-name="text">
          <p className="leading-[20px] relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] w-full" data-node-id="13:1289">
            Act 2 complete
          </p>
          <p className="leading-[16px] relative shrink-0 text-[12px] text-[color:var(--qt-text-secondary,#bdb6a8)] tracking-[0.024px] w-full" data-node-id="13:1290">
            Next: Act 3 · 1 worth-it quest available
          </p>
        </div>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2621" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
