// Reference only, never imported. Figma node 11:651, file OCm0ix5nud2HU18LNYKf3g, pulled 2026-10-25.
const assetPathPrefix = "https://www.figma.com/api/mcp/asset/95356970-cb03-4205-b2b1-5d1c07f84b14";
const imgChamferBg = `${assetPathPrefix}/d14f6.svg`;
const imgChamferBg1 = `${assetPathPrefix}/f1d85.svg`;
const imgIcon = `${assetPathPrefix}/a3565.svg`;
const imgChamferStroke = `${assetPathPrefix}/b9e9e.svg`;

export default function CollapsedBadgeDockedRight() {
  return (
    <div className="drop-shadow-[0px_4px_6px_rgba(0,0,0,0.5),0px_0px_0px_rgba(0,0,0,0.63)] relative size-full" data-node-id="11:651" data-name="CollapsedBadge (docked right)">
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="I11:651;24:629" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-bg">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferBg1} />
      </div>
      <div className="absolute left-[10px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-10px_-17px] mask-size-[48px_48px] size-[14px] top-[17px]" data-node-id="I11:651;6:247" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="icon">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgIcon} />
      </div>
      <p className="[word-break:break-word] absolute font-['Alegreya_Sans:ExtraBold'] h-[24px] leading-[24px] left-[27px] mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-position-[-27px_-12px] mask-size-[48px_48px] not-italic text-[22px] text-[color:var(--qt-worth-gold,#d9b45a)] top-[12px] w-[16px]" data-node-id="I11:651;19:3092" style={{ maskImage: `url("${imgChamferBg}")` }}>
        3
      </p>
      <div className="absolute inset-0 mask-alpha mask-intersect mask-no-clip mask-no-repeat mask-size-[100%_100%]" data-node-id="I11:651;24:630" style={{ maskImage: `url("${imgChamferBg}")` }} data-name="chamfer-stroke">
        <img alt="" className="absolute block inset-0 max-w-none size-full" src={imgChamferStroke} />
      </div>
    </div>
  );
}
