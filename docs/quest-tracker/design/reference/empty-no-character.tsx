// Reference only, never imported. Figma node 13:1233, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/358c807c-2650-4a39-a9ba-f61924171c15";
const imgIcon = `${assetPathPrefix}/5861c.svg`;
const imgChamferBg = `${assetPathPrefix}/b1468.svg`;
const imgChamferBg1 = `${assetPathPrefix}/db114.svg`;
const imgIcon1 = `${assetPathPrefix}/32460.svg`;
const imgIcon2 = `${assetPathPrefix}/9ee59.svg`;
const imgIcon3 = `${assetPathPrefix}/06523.svg`;
const imgChamferBg2 = `${assetPathPrefix}/52bc8.svg`;
const imgChamferBg3 = `${assetPathPrefix}/ae6e0.svg`;
const imgChamferStroke = `${assetPathPrefix}/0756c.svg`;

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

export default function EmptyNoCharacter() {
  return (
    <div className="content-stretch drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] flex flex-col items-start pb-[8px] relative size-full" data-node-id="13:1233" data-name="empty-no-character">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2617" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <div className="absolute inset-[-0.23%_-0.16%]">
          <img alt="" className="block max-w-none size-full" src={imgChamferBg1} />
        </div>
      </div>
      <div className="bg-[var(--qt-edit-tint,#0f3236)] content-stretch flex gap-[6px] h-[36px] items-center mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-0.5px] mask-size-[321px_217px] pl-[12px] pr-[6px] relative shrink-0 w-full" data-node-id="13:1234" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="PanelHeader">
        <div className="bg-[var(--qt-edit-accent,#3ccfcf)] content-stretch flex gap-[3px] items-center overflow-clip pl-[5px] pr-[6px] py-[2px] relative shrink-0" data-node-id="I13:1234;24:631" data-name="edit-chip">
          <div className="relative shrink-0 size-[12px]" data-node-id="I13:1234;24:632" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon1} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:ExtraBold'] leading-[14px] not-italic relative shrink-0 text-[12px] text-[color:var(--qt-panel-sunken,#0c0d0f)] tracking-[0.72px] whitespace-nowrap" data-node-id="I13:1234;24:635">
            EDIT
          </p>
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[20px] not-italic overflow-hidden relative shrink-0 text-[16px] text-[color:var(--qt-text-muted,#959084)] text-ellipsis whitespace-nowrap" data-node-id="I13:1234;7:205">
          Quest tracker
        </p>
        <div className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" data-node-id="I13:1234;7:212" data-name="collapse">
          <div className="relative shrink-0 size-[16px]" data-node-id="I13:1234;7:212;7:167" data-name="icon">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon2} />
          </div>
        </div>
        <IconButton className="content-stretch flex items-center justify-center relative shrink-0 size-[24px]" />
      </div>
      <div className="content-stretch flex flex-col gap-[8px] items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-36.5px] mask-size-[321px_217px] overflow-clip px-[16px] py-[20px] relative shrink-0 w-full" data-node-id="13:1254" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="empty">
        <div className="relative shrink-0 size-[24px]" data-node-id="13:1255" data-name="icon">
          <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon3} />
        </div>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[20px] min-w-full not-italic relative shrink-0 text-[16px] text-[color:var(--qt-text-primary,#ece6d9)] w-[min-content]" data-node-id="13:1259">
          No character yet
        </p>
        <p className="[word-break:break-word] font-['Alegreya_Sans:Regular'] leading-[18px] min-w-full not-italic relative shrink-0 text-[14px] text-[color:var(--qt-text-secondary,#bdb6a8)] w-[min-content]" data-node-id="13:1260">
          Enter any zone and the character is picked up from the log, or add one by hand.
        </p>
      </div>
      <div className="content-stretch flex items-start mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-0.5px_-172.5px] mask-size-[321px_217px] overflow-clip pb-[8px] px-[16px] relative shrink-0 w-full" data-node-id="13:1261" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="action">
        <div className="content-stretch flex gap-[var(--qt-space-2,8px)] h-[28px] items-center justify-center px-[12px] relative shrink-0 w-[97px]" data-node-id="13:1262" data-name="Button">
          <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="I13:1262;24:581" style={{ maskImage: `url("${imgChamferBg2}")` }} data-name="chamfer-bg">
            <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferBg3} />
          </div>
          <p className="[word-break:break-word] font-['Alegreya_Sans:Bold'] leading-[16px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-12px_-6px] mask-size-[80px_28px] not-italic relative shrink-0 text-[13px] text-[color:var(--qt-text-on-light,#121416)] tracking-[0.065px] whitespace-nowrap" data-node-id="I13:1262;6:149" style={{ maskImage: `url("${imgChamferBg2}")` }}>
            Add character
          </p>
        </div>
      </div>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="24:2618" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
