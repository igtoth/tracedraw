// The object model VBA macros for drawing suites use, for JavaScript
// compiled from VBA (see vg.rs and docs/behavior/vba.md). Lengths follow
// ActiveDocument.Unit (inches unless a macro sets it); the host works in
// page millimetres, y up from the page's lower left corner.

// ----- Constants ----------------------------------------------------------

// docs/behavior/vba.md lists which enumerations were checked against the
// published reference.
const __consts = {
  // cdrUnit
  cdrTenthMicron: 0, cdrInch: 1, cdrFoot: 2, cdrMillimeter: 3, cdrCentimeter: 4,
  cdrPixel: 5, cdrMile: 6, cdrMeter: 7, cdrKilometer: 8, cdrDidots: 9, cdrAgate: 10,
  cdrYard: 11, cdrPica: 12, cdrCicero: 13, cdrPoint: 14, cdrUnitQ: 15, cdrUnitH: 16,
  // cdrReferencePoint
  cdrTopRight: 1, cdrTopMiddle: 2, cdrTopLeft: 3, cdrMiddleLeft: 4, cdrBottomLeft: 5,
  cdrBottomMiddle: 6, cdrBottomRight: 7, cdrMiddleRight: 8, cdrCenter: 9,
  // cdrShapeType
  cdrNoShape: 0, cdrRectangleShape: 1, cdrEllipseShape: 2, cdrCurveShape: 3,
  cdrPolygonShape: 4, cdrBitmapShape: 5, cdrTextShape: 6, cdrGroupShape: 7,
  cdrSelectionShape: 8, cdrGuidelineShape: 9, cdrBlendGroupShape: 10,
  cdrExtrudeGroupShape: 11, cdrOLEObjectShape: 12, cdrContourGroupShape: 13,
  cdrLinearDimensionShape: 14, cdrBevelGroupShape: 15, cdrDropShadowGroupShape: 16,
  cdr3DObjectShape: 17, cdrArtisticMediaGroupShape: 18, cdrConnectorShape: 19,
  cdrMeshFillShape: 20, cdrCustomShape: 21, cdrCustomEffectGroupShape: 22,
  cdrSymbolShape: 23, cdrHTMLFormObjectShape: 24, cdrHTMLActiveObjectShape: 25,
  cdrPerfectShape: 26, cdrEPSShape: 27,
  // cdrFillType
  cdrNoFill: 0, cdrUniformFill: 1, cdrFountainFill: 2, cdrPostscriptFill: 3,
  cdrTextureFill: 8, cdrPatternFill: 9, cdrHatchFill: 10,
  // cdrFountainFillType
  cdrLinearFountainFill: 1, cdrRadialFountainFill: 2, cdrConicalFountainFill: 3,
  cdrSquareFountainFill: 4,
  // cdrColorType
  cdrColorPantone: 1, cdrColorCMYK: 2, cdrColorCMY: 4, cdrColorRGB: 5, cdrColorHSB: 6,
  cdrColorHLS: 7, cdrColorBlackAndWhite: 8, cdrColorGray: 9, cdrColorYIQ: 11,
  cdrColorLab: 12, cdrColorPantoneHex: 14, cdrColorRegistration: 20, cdrColorUserInk: 22,
  cdrColorSpot: 25, cdrColorMultiChannel: 26, cdrColorMixed: 99,
  // cdrOutlineType
  cdrNoOutline: 0, cdrOutline: 1, cdrEnhancedOutline: 2,
  // cdrOutlineLineJoin
  cdrOutlineMiterLineJoin: 0, cdrOutlineRoundLineJoin: 1, cdrOutlineBevelLineJoin: 2,
  // cdrOutlineLineCaps
  cdrOutlineUndefinedLineCaps: -1, cdrOutlineButtLineCaps: 0, cdrOutlineRoundLineCaps: 1,
  cdrOutlineSquareLineCaps: 2,
  // cdrFlipAxes
  cdrFlipHorizontal: 1, cdrFlipVertical: 2, cdrFlipBoth: 3,
  // cdrAlignType
  cdrAlignLeft: 1, cdrAlignRight: 2, cdrAlignHCenter: 3, cdrAlignTop: 4,
  cdrAlignBottom: 8, cdrAlignVCenter: 12,
  // cdrAlignDistributeH
  cdrAlignDistributeHNone: 0, cdrAlignDistributeHAlignRight: 1, cdrAlignDistributeHAlignLeft: 2,
  cdrAlignDistributeHAlignCenter: 3, cdrAlignDistributeHDistributeRight: 4,
  cdrAlignDistributeHDistributeLeft: 5, cdrAlignDistributeHDistributeCenter: 6,
  cdrAlignDistributeHDistributeSpacing: 7,
  // cdrAlignDistributeV
  cdrAlignDistributeVNone: 0, cdrAlignDistributeVAlignTop: 1, cdrAlignDistributeVAlignBottom: 2,
  cdrAlignDistributeVAlignCenter: 3, cdrAlignDistributeVDistributeTop: 4,
  cdrAlignDistributeVDistributeBottom: 5, cdrAlignDistributeVDistributeCenter: 6,
  cdrAlignDistributeVDistributeSpacing: 7,
  // cdrAlignShapesTo
  cdrAlignShapesToLastSelected: 0, cdrAlignShapesToEdgeOfPage: 1,
  cdrAlignShapesToCenterOfPage: 2, cdrAlignShapesToGrid: 3, cdrAlignShapesToPoint: 4,
  // cdrDistributeArea
  cdrDistributeToSelection: 0, cdrDistributeToPage: 1, cdrDistributeToRect: 2,
  // cdrAlignment (text)
  cdrNoAlignment: 0, cdrLeftAlignment: 1, cdrRightAlignment: 2, cdrCenterAlignment: 3,
  cdrFullJustifyAlignment: 4, cdrForceJustifyAlignment: 5, cdrMixedAlignment: 6,
  // cdrTextType
  cdrArtisticText: 0, cdrParagraphText: 1, cdrArtisticFittedText: 2, cdrParagraphFittedText: 3,
  // cdrFontLine
  cdrNoFontLine: 0, cdrSingleThinFontLine: 1, cdrSingleThinWordFontLine: 2,
  cdrSingleThickFontLine: 3, cdrSingleThickWordFontLine: 4, cdrDoubleThinFontLine: 5,
  cdrDoubleThinWordFontLine: 6, cdrMixedFontLine: 7,
  // cdrFontCase
  cdrNormalFontCase: 0, cdrSmallCapsFontCase: 1, cdrAllCapsFontCase: 2, cdrMixedFontCase: 3,
  // cdrNodeType
  cdrCuspNode: 0, cdrSmoothNode: 1, cdrSymmetricalNode: 2,
  // cdrSegmentType
  cdrLineSegment: 0, cdrCurveSegment: 1, cdrMixedSegments: 2,
  // cdrPageOrientation
  cdrPortrait: 0, cdrLandscape: 1,
  // cdrExportRange
  cdrCurrentPage: 0, cdrAllPages: 1, cdrSelection: 2, cdrPageRange: 3,
  // cdrFilter
  cdrAutoSense: 0, cdrBMP: 769, cdrPCX: 770, cdrTGA: 771, cdrTIFF: 772, cdrGIF: 773,
  cdrJPEG: 774, cdrPCD: 775, cdrSCT: 776, cdrICO: 784, cdrCUR: 785, cdrEXE: 786,
  cdrIMG: 787, cdrPSD: 788, cdrPP4: 789, cdrMAC: 791, cdrOS2BMP: 792, cdrWI: 793,
  cdrWVL: 793, cdrCAL: 800, cdrDCS: 801, cdrPNG: 802, cdrPP5: 803, cdrEPSPhotoPaint: 804,
  cdrFPX: 806, cdrRIFF: 807, cdrXPM: 809, cdrXCF: 816, cdrJPG2000Import: 818, cdrPPF: 819,
  cdrJP2: 820, cdrJPC: 821, cdrPSP: 822, cdrRAW: 823, cdrWEBP: 825, cdrCGM: 1280,
  cdrHPGL: 1281, cdrPLT: 1281, cdrGEM: 1284, cdrWPG: 1287, cdrPSEncapsulated: 1289,
  cdrEPS: 1289, cdrPSInterpreted: 1290, cdrMET: 1291, cdrNAP: 1292, cdrPICT: 1293,
  cdrWMF: 1294, cdrDXF: 1296, cdrEMF: 1300, cdrTTF: 1302, cdrAT1: 1303, cdrAI: 1305,
  cdrVSD: 1315, cdrDWG: 1328, cdrFMV: 1329, cdrPDF: 1333, cdrAI9: 1333, cdrPIC: 1334,
  cdrDRW: 1339, cdrDSF: 1339, cdrSWF: 1343, cdrFH: 1344, cdrSVG: 1345, cdrSVGZ: 1347,
  cdrPUB: 1349, cdrAVI: 1536, cdrQTM: 1542, cdrPPT: 1548, cdrHTML: 1557,
  cdrGIFAnimation: 1558, cdr3DMF: 1559, cdrQTVR: 1560, cdrCMX6: 1793, cdrCMX5: 1794,
  cdrCDR: 1795, cdrCDX: 1796, cdrCPX: 1797, cdrCPT8: 1799, cdrCDT: 1800, cdrPAT: 1801,
  cdrCLK: 1802, cdrDES: 1805, cdrSHW: 1806, cdrCPT: 1808, cdrCPT9: 1808, cdrCPT10: 1808,
  cdrCPT11: 1808, cdrCSL: 1810, cdrCMX64: 1811, cdrTXT: 2048, cdrWord95: 2049,
  cdrWord2: 2050, cdrWord55: 2051, cdrMacWord5: 2052, cdrRTF: 2053, cdrWP9: 2055,
  cdrWPD: 2055, cdrWP51: 2056, cdrWP50: 2057, cdrWP4: 2058, cdrWSW: 2059, cdrWS7: 2060,
  cdrWS2000: 2061, cdrWSD: 2061, cdrXY: 2062, cdrSAM: 2063, cdrXLS: 2064, cdrWKS: 2066,
  cdrQuattroPro: 2067, cdrDOC: 2068, cdrWord2000: 2068, cdrWS6: 2163, cdrWS4: 2164,
  cdrMIF: 2165, cdrWPM: 2166, cdrCGZ: 2315,
  // cdrImageType
  cdrBlackAndWhiteImage: 0, cdr16ColorsImage: 1, cdrGrayscaleImage: 2,
  cdrPalettedImage: 3, cdrRGBColorImage: 4, cdrCMYKColorImage: 5, cdrDuotoneImage: 6,
  cdrLABImage: 7, cdrCMYKMultiChannelImage: 8, cdrRGBMultiChannelImage: 9,
  cdrSpotMultiChannelImage: 10,
  // cdrTriState
  cdrTrue: -1, cdrFalse: 0, cdrUndefined: -2,
  // cdrTransparencyType
  cdrNoTransparency: 0, cdrUniformTransparency: 1, cdrFountainTransparency: 2,
  cdrPatternTransparency: 3, cdrTextureTransparency: 4,
  // cdrTransparencyAppliedTo
  cdrApplyToFill: 0, cdrApplyToOutline: 1, cdrApplyToFillAndOutline: 2,
  // cdrMergeMode
  cdrMergeNormal: 0, cdrMergeAND: 1, cdrMergeOR: 2, cdrMergeXOR: 3, cdrMergeInvert: 6,
  cdrMergeAdd: 7, cdrMergeSubtract: 8, cdrMergeMultiply: 9, cdrMergeDivide: 10,
  cdrMergeIfLighter: 11, cdrMergeIfDarker: 12, cdrMergeTexturize: 13, cdrMergeColor: 14,
  cdrMergeHue: 15, cdrMergeSaturation: 16, cdrMergeLightness: 17, cdrMergeRed: 18,
  cdrMergeGreen: 19, cdrMergeBlue: 20, cdrMergeDifference: 24, cdrMergeBehind: 27,
  cdrMergeScreen: 28, cdrMergeOverlay: 29, cdrMergeSoftlight: 30, cdrMergeHardlight: 31,
  cdrMergeDodge: 33, cdrMergeBurn: 34, cdrMergeExclusion: 36,
  // cdrEffectType
  cdrBlend: 0, cdrExtrude: 1, cdrEnvelope: 2, cdrTextOnPath: 3, cdrControlPath: 4,
  cdrDropShadow: 5, cdrContour: 6, cdrDistortion: 7, cdrPerspective: 8, cdrLens: 9,
  cdrCustomEffect: 10, cdrInnerShadow: 11,
  // cdrLensType
  cdrLensBrighten: 0, cdrLensColorAdd: 1, cdrLensColorLimit: 2, cdrLensCustomColorMap: 3,
  cdrLensFishEye: 4, cdrLensHeatMap: 5, cdrLensInvert: 6, cdrLensMagnify: 7,
  cdrLensTintedGrayscale: 8, cdrLensTransparency: 9, cdrLensWireframe: 10,
  // cdrViewType
  cdrSimpleWireframeView: 0, cdrWireframeView: 1, cdrDraftView: 2, cdrNormalView: 3,
  cdrEnhancedView: 4, cdrPixelView: 6,
  // cdrAntiAliasingType
  cdrNoAntiAliasing: 0, cdrNormalAntiAliasing: 1, cdrSupersampling: 2,
  // cdrTextLanguage (Windows language ids)
  cdrLanguageNone: 0, cdrEnglishUS: 1033, cdrEnglishUK: 2057, cdrRussian: 1049,
  cdrGerman: 1031, cdrFrench: 1036, cdrSpanish: 1034, cdrItalian: 1040,
  cdrPortuguese: 2070, cdrBrazilianPortuguese: 1046,
  // cdrImportMode
  cdrImportFull: 0,
  // cdrCursorShape
  cdrCursorWinAppStarting: 0, cdrCursorWinArrow: 1, cdrCursorWinCross: 2,
  cdrCursorWinHelp: 3, cdrCursorWinIbeam: 4, cdrCursorSmallcrosshair: 301,
  cdrCursorPick: 305, cdrCursorPickNone: 305, cdrCursorEyeDrop: 326,
  cdrCursorExtPick: 351, cdrCursorPickOvertarget: 396,
  // cdrFileVersion
  cdrCurrentVersion: 0,
  // cdrShapeLinkType
  cdrLinkAllConnections: 0,
  // cdrCompressionType
  cdrCompressionNone: 0, cdrCompressionLZW: 1,
};
for (const k in __consts) globalThis[k] = __consts[k];

const __filterExt = { [cdrCDR]: 'cdr', [cdrPNG]: 'png', [cdrJPEG]: 'jpg', [cdrBMP]: 'bmp',
  [cdrTIFF]: 'tif', [cdrGIF]: 'gif', [cdrPDF]: 'pdf', [cdrSVG]: 'svg', [cdrEPS]: 'eps',
  [cdrPSInterpreted]: 'ps', [cdrWEBP]: 'webp' };

// ----- Units and geometry -------------------------------------------------

const __vg = { unit: cdrInch, ref: cdrCenter, rot: new Map(), optimization: false,
  events: true, groupDepth: 0 };
const __mmPer = { 0: 0.0001, 1: 25.4, 2: 304.8, 3: 1, 4: 10, 5: 25.4 / 96, 6: 1609344,
  7: 1000, 8: 1000000, 9: 0.376065, 10: 25.4 / 14, 11: 914.4, 12: 25.4 / 6, 13: 4.51165,
  14: 25.4 / 72, 15: 0.25, 16: 0.25 };
function __f() { return __mmPer[__vg.unit] || 25.4; }
function __mm(v) { return Number(v) * __f(); }
function __un(mm) { return mm / __f(); }
function __ids(x) {
  if (x === undefined || x === null) return [];
  if (typeof x._ids === 'function') return x._ids();
  if (Array.isArray(x)) return x.flatMap(__ids);
  return [];
}
function __refPoint(b, ref) {
  const cx = (b.x0 + b.x1) / 2, cy = (b.y0 + b.y1) / 2;
  switch (ref === undefined ? __vg.ref : ref) {
    case 1: return [b.x1, b.y1];
    case 2: return [cx, b.y1];
    case 3: return [b.x0, b.y1];
    case 4: return [b.x0, cy];
    case 5: return [b.x0, b.y0];
    case 6: return [cx, b.y0];
    case 7: return [b.x1, b.y0];
    case 8: return [b.x1, cy];
    default: return [cx, cy];
  }
}
const __M = {
  move: (dx, dy) => [1, 0, 0, 1, dx, dy],
  scale: (cx, cy, sx, sy) => [sx, 0, 0, sy, cx - sx * cx, cy - sy * cy],
  rotate: (deg, cx, cy) => {
    const r = deg * Math.PI / 180, c = Math.cos(r), s = Math.sin(r);
    return [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy];
  },
  skew: (ax, ay, cx, cy) => {
    const kx = Math.tan(ax * Math.PI / 180), ky = Math.tan(ay * Math.PI / 180);
    return [1, ky, kx, 1, -kx * cy, -ky * cx];
  },
};
function __xf(ids, m) { if (ids.length) __call('vg_transform', { ids: ids, m: m }); }
function __iter(list) { return list[Symbol.iterator](); }

// ----- Color --------------------------------------------------------------

const __colorType = { rgb: cdrColorRGB, cmyk: cdrColorCMYK, gray: cdrColorGray, hsb: cdrColorHSB,
  hsl: cdrColorHLS, lab: cdrColorLab, yiq: cdrColorYIQ, registration: cdrColorRegistration,
  spot: cdrColorSpot };
class Color {
  constructor(v, bind) { this.v = v || { model: 'rgb', r: 0, g: 0, b: 0 }; this._bind = bind || null; }
  _set(v) { this.v = v; if (this._bind) this._bind(v); }
  _conv(model) { return __call('vg_color', { color: this.v, model: model }); }
  get Type() { return __colorType[this.v.model] || cdrColorRGB; }
  RGBAssign(Red, Green, Blue) { this._set({ model: 'rgb', r: Red / 255, g: Green / 255, b: Blue / 255 }); }
  CMYKAssign(Cyan, Magenta, Yellow, Black) {
    this._set({ model: 'cmyk', c: Cyan / 100, m: Magenta / 100, y: Yellow / 100, k: Black / 100 });
  }
  CMYAssign(Cyan, Magenta, Yellow) { this.CMYKAssign(Cyan * 100 / 255, Magenta * 100 / 255, Yellow * 100 / 255, 0); }
  GrayAssign(GrayValue) { this._set({ model: 'gray', v: GrayValue / 255 }); }
  BWAssign(White) { this._set({ model: 'gray', v: White ? 1 : 0 }); }
  HSBAssign(Hue, Saturation, Brightness) {
    this._set({ model: 'hsb', h: Hue, s: Saturation / 100, b: Brightness / 100 });
  }
  HLSAssign(Hue, Lightness, Saturation) {
    this._set({ model: 'hsl', h: Hue, s: Saturation / 100, l: Lightness / 100 });
  }
  LabAssign(L, A, B) { this._set({ model: 'lab', l: L, a: A, b: B }); }
  RegistrationAssign() { this._set({ model: 'registration' }); }
  CopyAssign(Color) { this._set(JSON.parse(JSON.stringify(Color.v))); }
  StringAssign(ColorString) { this._set(__parseColor(ColorString).v); }
  ConvertToRGB() { this._set(this._conv('RGB').color); }
  ConvertToCMYK() { this._set(this._conv('CMYK').color); }
  ConvertToGray() { this._set(this._conv('Grayscale').color); }
  ConvertToHSB() { this._set(this._conv('HSB').color); }
  ConvertTo(ColorModel) {
    const m = { 2: 'CMYK', 5: 'RGB', 6: 'HSB', 7: 'HSL', 9: 'Grayscale', 12: 'Lab', 11: 'YIQ' }[ColorModel];
    if (m) this._set(this._conv(m).color);
  }
  get _rgb() { return this._conv('RGB').rgb; }
  get RGBRed() { return this._rgb[0]; }
  get RGBGreen() { return this._rgb[1]; }
  get RGBBlue() { return this._rgb[2]; }
  set RGBRed(v) { const c = this._rgb; this.RGBAssign(v, c[1], c[2]); }
  set RGBGreen(v) { const c = this._rgb; this.RGBAssign(c[0], v, c[2]); }
  set RGBBlue(v) { const c = this._rgb; this.RGBAssign(c[0], c[1], v); }
  get RGBValue() { const c = this._rgb; return c[0] + c[1] * 256 + c[2] * 65536; }
  set RGBValue(v) { this.RGBAssign(v & 255, (v >> 8) & 255, (v >> 16) & 255); }
  get _cmyk() { const c = this.v.model === 'cmyk' ? this.v : this._conv('CMYK').color; return c; }
  get CMYKCyan() { return Math.round(this._cmyk.c * 100); }
  get CMYKMagenta() { return Math.round(this._cmyk.m * 100); }
  get CMYKYellow() { return Math.round(this._cmyk.y * 100); }
  get CMYKBlack() { return Math.round(this._cmyk.k * 100); }
  set CMYKCyan(v) { const c = this._cmyk; this.CMYKAssign(v, c.m * 100, c.y * 100, c.k * 100); }
  set CMYKMagenta(v) { const c = this._cmyk; this.CMYKAssign(c.c * 100, v, c.y * 100, c.k * 100); }
  set CMYKYellow(v) { const c = this._cmyk; this.CMYKAssign(c.c * 100, c.m * 100, v, c.k * 100); }
  set CMYKBlack(v) { const c = this._cmyk; this.CMYKAssign(c.c * 100, c.m * 100, c.y * 100, v); }
  get Gray() { const c = this.v.model === 'gray' ? this.v : this._conv('Grayscale').color; return Math.round(c.v * 255); }
  get HSBHue() { return this._conv('HSB').color.h; }
  get HSBSaturation() { return Math.round(this._conv('HSB').color.s * 100); }
  get HSBBrightness() { return Math.round(this._conv('HSB').color.b * 100); }
  get HexValue() { return this._conv('RGB').hex; }
  set HexValue(h) { this._set(__parseColor(h).v); }
  get Name() { return this.v.model === 'spot' ? 'Spot ' + this.v.ink : ''; }
  get IsSpot() { return this.v.model === 'spot'; }
  get IsCMYK() { return this.v.model === 'cmyk'; }
  IsSame(Color) {
    const a = this._rgb, b = Color._rgb;
    return a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
  }
  toString() { return this.HexValue; }
}
function __parseColor(s) {
  s = String(s || '').trim();
  const c = new Color();
  let m;
  if ((m = /^#?([0-9a-f]{6})$/i.exec(s))) {
    const n = parseInt(m[1], 16);
    c.RGBAssign((n >> 16) & 255, (n >> 8) & 255, n & 255);
  } else if ((m = /^RGB\d*,[^,]*,(\d+),(\d+),(\d+)/i.exec(s))) {
    c.RGBAssign(+m[1], +m[2], +m[3]);
  } else if ((m = /^CMYK\d*,[^,]*,(\d+),(\d+),(\d+),(\d+)/i.exec(s))) {
    c.CMYKAssign(+m[1], +m[2], +m[3], +m[4]);
  } else if ((m = /^GRAY\d*,[^,]*,(\d+)/i.exec(s))) {
    c.GrayAssign(+m[1]);
  }
  return c;
}

// ----- Fill, outline, transparency ---------------------------------------

const __fillType = { none: cdrNoFill, solid: cdrUniformFill, fountain: cdrFountainFill,
  pattern: cdrPatternFill, texture: cdrTextureFill, mesh: cdrUniformFill };
function __plain(f) { const o = Object.assign({}, f); delete o.fill; return o; }
class Fill {
  constructor(owner) { this.o = owner; }
  get _f() { const ids = this.o._ids(); return ids.length ? __call('vg_info', { id: ids[0] }).fill : { fill: 'none' }; }
  _put(fill) { __call('vg_set_fill', { ids: this.o._ids(), fill: fill }); }
  get Type() { return __fillType[this._f.fill] === undefined ? cdrNoFill : __fillType[this._f.fill]; }
  get UniformColor() {
    const f = this._f;
    const v = f.fill === 'solid' ? __plain(f) : { model: 'rgb', r: 0, g: 0, b: 0 };
    return new Color(v, (nv) => this._put(Object.assign({ fill: 'solid' }, nv)));
  }
  set UniformColor(c) { this.ApplyUniformFill(c); }
  ApplyNoFill() { this._put({ fill: 'none' }); }
  ApplyUniformFill(Color) { this._put(Object.assign({ fill: 'solid' }, Color.v)); }
  ApplyFountainFill(StartColor, EndColor, Type, Angle, Steps, EdgePad, MidPoint, BlendType, CenterOffsetX, CenterOffsetY) {
    const from = (StartColor || new Color()).v;
    const to = (EndColor || new Color({ model: 'rgb', r: 1, g: 1, b: 1 })).v;
    for (const id of this.o._ids()) __call('set_fountain', { id: id, from: from, to: to, angle: Angle || 0 });
  }
  CopyAssign(SourceFill) { this._put(SourceFill._f); }
}
function __defaultStroke() { return __call('vg_default_stroke', {}); }
class OutlineStyle {
  constructor(outline) { this.ol = outline; }
  get DashCount() { const s = this.ol._s; return s && s.dash ? s.dash.length / 2 : 0; }
  get DashLength() { const s = this.ol._s; return s && s.dash && s.dash.length ? s.dash[0] : 0; }
  GetDashAndGapLength(Index) { const s = this.ol._s; return s && s.dash ? [s.dash[2 * (Index - 1)], s.dash[2 * (Index - 1) + 1]] : [0, 0]; }
  SetDashes(...lengths) { const s = this.ol._s || __defaultStroke(); s.dash = lengths.map(Number); this.ol._put(s); }
}
class Outline {
  constructor(owner) { this.o = owner; }
  get _s() { const ids = this.o._ids(); return ids.length ? __call('vg_info', { id: ids[0] }).stroke : null; }
  _put(st) { __call('vg_set_stroke', { ids: this.o._ids(), stroke: st }); }
  get Type() { return this._s ? cdrOutline : cdrNoOutline; }
  set Type(t) { if (t === cdrNoOutline) this.SetNoOutline(); else if (!this._s) this._put(__defaultStroke()); }
  get Width() { const s = this._s; return s ? __un(s.width) : 0; }
  set Width(w) { const s = this._s || __defaultStroke(); s.width = __mm(w); this._put(s); }
  get Color() {
    const s = this._s;
    return new Color(s ? s.color : { model: 'rgb', r: 0, g: 0, b: 0 },
      (nv) => { const t = this._s || __defaultStroke(); t.color = nv; this._put(t); });
  }
  set Color(c) { const s = this._s || __defaultStroke(); s.color = c.v; this._put(s); }
  get Style() { return new OutlineStyle(this); }
  get LineCaps() { const s = this._s; return s ? ({ butt: 0, round: 1, square: 2 }[s.cap] || 0) : 0; }
  set LineCaps(v) { const s = this._s || __defaultStroke(); s.cap = ['butt', 'round', 'square'][v] || 'butt'; this._put(s); }
  get LineJoin() { const s = this._s; return s ? ({ miter: 0, round: 1, bevel: 2 }[s.join] || 0) : 0; }
  set LineJoin(v) { const s = this._s || __defaultStroke(); s.join = ['miter', 'round', 'bevel'][v] || 'miter'; this._put(s); }
  get StartArrow() { return null; }
  set StartArrow(v) { }
  get EndArrow() { return null; }
  set EndArrow(v) { }
  SetNoOutline() { this._put(null); }
  SetProperties(Width, Style, Color) {
    const s = this._s || __defaultStroke();
    if (Width !== undefined && Width !== null && Width >= 0) s.width = __mm(Width);
    if (Color && Color.v) s.color = Color.v;
    this._put(s);
  }
  CopyAssign(SourceOutline) { this._put(SourceOutline._s); }
}
class Transparency {
  constructor(owner) { this.o = owner; }
  get Type() { return this.Uniform > 0 ? 1 : 0; }
  get Uniform() { const ids = this.o._ids(); return ids.length ? Math.round((1 - __call('vg_info', { id: ids[0] }).opacity) * 100) : 0; }
  set Uniform(v) { this.ApplyUniformTransparency(v); }
  ApplyUniformTransparency(Value) { __call('vg_props', { ids: this.o._ids(), opacity: 1 - Value / 100 }); }
  ApplyNoTransparency() { __call('vg_props', { ids: this.o._ids(), opacity: 1 }); }
}

// ----- Geometry shared by shapes, ranges and the selection ---------------

class __Geo {
  _b() { return __call('vg_bounds', { ids: this._ids() }); }
  _rp(ref) { return __refPoint(this._b(), ref); }
  get PositionX() { return __un(this._rp()[0]); }
  get PositionY() { return __un(this._rp()[1]); }
  set PositionX(v) { this.SetPosition(v, this.PositionY); }
  set PositionY(v) { this.SetPosition(this.PositionX, v); }
  get SizeWidth() { const b = this._b(); return __un(b.x1 - b.x0); }
  get SizeHeight() { const b = this._b(); return __un(b.y1 - b.y0); }
  set SizeWidth(v) { this.SetSize(v, this.SizeHeight); }
  set SizeHeight(v) { this.SetSize(this.SizeWidth, v); }
  get LeftX() { return __un(this._b().x0); }
  get RightX() { return __un(this._b().x1); }
  get TopY() { return __un(this._b().y1); }
  get BottomY() { return __un(this._b().y0); }
  get CenterX() { const b = this._b(); return __un((b.x0 + b.x1) / 2); }
  get CenterY() { const b = this._b(); return __un((b.y0 + b.y1) / 2); }
  set LeftX(v) { __xf(this._ids(), __M.move(__mm(v) - this._b().x0, 0)); }
  set RightX(v) { __xf(this._ids(), __M.move(__mm(v) - this._b().x1, 0)); }
  set TopY(v) { __xf(this._ids(), __M.move(0, __mm(v) - this._b().y1)); }
  set BottomY(v) { __xf(this._ids(), __M.move(0, __mm(v) - this._b().y0)); }
  set CenterX(v) { const b = this._b(); __xf(this._ids(), __M.move(__mm(v) - (b.x0 + b.x1) / 2, 0)); }
  set CenterY(v) { const b = this._b(); __xf(this._ids(), __M.move(0, __mm(v) - (b.y0 + b.y1) / 2)); }
  GetPosition() { return [this.PositionX, this.PositionY]; }
  GetSize() { return [this.SizeWidth, this.SizeHeight]; }
  GetBoundingBox(x, y, Width, Height, UseOutline) {
    const b = this._b();
    return [__un(b.x0), __un(b.y0), __un(b.x1 - b.x0), __un(b.y1 - b.y0)];
  }
  SetPosition(PositionX, PositionY) {
    const p = this._rp();
    __xf(this._ids(), __M.move(__mm(PositionX) - p[0], __mm(PositionY) - p[1]));
  }
  SetSize(Width, Height) {
    const b = this._b(), w = b.x1 - b.x0, h = b.y1 - b.y0;
    let sx = Width ? __mm(Width) / (w || 1e-9) : 0;
    let sy = Height ? __mm(Height) / (h || 1e-9) : 0;
    if (!sx && !sy) return;
    if (!sx) sx = sy;
    if (!sy) sy = sx;
    const p = __refPoint(b);
    __xf(this._ids(), __M.scale(p[0], p[1], sx, sy));
  }
  SetSizeEx(CenterX, CenterY, Width, Height) {
    const b = this._b(), w = b.x1 - b.x0, h = b.y1 - b.y0;
    let sx = Width ? __mm(Width) / (w || 1e-9) : 0;
    let sy = Height ? __mm(Height) / (h || 1e-9) : 0;
    if (!sx) sx = sy || 1;
    if (!sy) sy = sx;
    __xf(this._ids(), __M.scale(__mm(CenterX), __mm(CenterY), sx, sy));
  }
  SetBoundingBox(x, y, Width, Height, KeepAspect, ReferencePoint) {
    const b = this._b(), w = b.x1 - b.x0, h = b.y1 - b.y0;
    let sx = __mm(Width) / (w || 1e-9), sy = __mm(Height) / (h || 1e-9);
    if (KeepAspect) { sx = sy = Math.min(sx, sy); }
    __xf(this._ids(), [sx, 0, 0, sy, __mm(x) - sx * b.x0, __mm(y) - sy * b.y0]);
  }
  Move(DeltaX, DeltaY) { __xf(this._ids(), __M.move(__mm(DeltaX || 0), __mm(DeltaY || 0))); }
  _center() { const b = this._b(); return [(b.x0 + b.x1) / 2, (b.y0 + b.y1) / 2]; }
  Rotate(Angle) { const c = this._rotCenter(); __xf(this._ids(), __M.rotate(Angle, c[0], c[1])); }
  RotateEx(Angle, CenterX, CenterY) { __xf(this._ids(), __M.rotate(Angle, __mm(CenterX), __mm(CenterY))); }
  _rotCenter() { return this._center(); }
  Skew(AngleX, AngleY) { const c = this._center(); __xf(this._ids(), __M.skew(AngleX || 0, AngleY || 0, c[0], c[1])); }
  SkewEx(AngleX, AngleY, CenterX, CenterY) { __xf(this._ids(), __M.skew(AngleX || 0, AngleY || 0, __mm(CenterX), __mm(CenterY))); }
  Stretch(StretchX, StretchY, StretchCharactersSize) {
    const p = this._rp();
    __xf(this._ids(), __M.scale(p[0], p[1], StretchX, StretchY === undefined ? StretchX : StretchY));
  }
  StretchEx(CenterX, CenterY, StretchX, StretchY) {
    __xf(this._ids(), __M.scale(__mm(CenterX), __mm(CenterY), StretchX, StretchY === undefined ? StretchX : StretchY));
  }
  Flip(Axes) {
    const c = this._center();
    __xf(this._ids(), __M.scale(c[0], c[1], Axes & 1 ? -1 : 1, Axes & 2 ? -1 : 1));
  }
  get Fill() { return new Fill(this); }
  get Outline() { return new Outline(this); }
  get Transparency() { return new Transparency(this); }
  ApplyUniformFill(Color) { this.Fill.ApplyUniformFill(Color); }
  ApplyNoFill() { this.Fill.ApplyNoFill(); }
  SetOutlineProperties(Width, Style, Color) { this.Outline.SetProperties(Width, Style, Color); }
  Delete() { __call('vg_delete', { ids: this._ids() }); }
  ConvertToCurves() { for (const id of this._ids()) __call('convert_to_curves', { id: id }); }
  AddToSelection() { __call('select', { ids: this._ids(), add: true }); }
  RemoveFromSelection() { const drop = this._ids(); __call('select', { ids: __call('selection', {}).ids.filter(i => !drop.includes(i)), add: false }); }
  CreateSelection() { __call('select', { ids: this._ids(), add: false }); }
  OrderToFront() { __call('vg_order', { ids: this._ids(), op: 'front' }); }
  OrderToBack() { __call('vg_order', { ids: this._ids().slice().reverse(), op: 'back' }); }
  OrderForwardOne() { __call('vg_order', { ids: this._ids().slice().reverse(), op: 'forward' }); }
  OrderBackOne() { __call('vg_order', { ids: this._ids(), op: 'backward' }); }
  OrderFrontOf(Shape) { __call('vg_order', { ids: this._ids(), op: 'front_of', ref: __ids(Shape)[0] }); }
  OrderBackOf(Shape) { __call('vg_order', { ids: this._ids(), op: 'back_of', ref: __ids(Shape)[0] }); }
  get Locked() { const ids = this._ids(); return ids.length > 0 && ids.every(id => __call('vg_info', { id: id }).locked); }
  set Locked(v) { __call('vg_props', { ids: this._ids(), locked: !!v }); }
  get Visible() { const ids = this._ids(); return ids.every(id => __call('vg_info', { id: id }).visible); }
  set Visible(v) { __call('vg_props', { ids: this._ids(), visible: !!v }); }
  get OverprintFill() { const ids = this._ids(); return ids.length > 0 && __call('vg_info', { id: ids[0] }).overprint_fill; }
  set OverprintFill(v) { __call('vg_props', { ids: this._ids(), overprint_fill: !!v }); }
  get OverprintOutline() { const ids = this._ids(); return ids.length > 0 && __call('vg_info', { id: ids[0] }).overprint_outline; }
  set OverprintOutline(v) { __call('vg_props', { ids: this._ids(), overprint_outline: !!v }); }
  AlignToShape(Type, Shape) { __align(this, Type, __call('vg_bounds', { ids: __ids(Shape) })); }
  AlignToPage(Type) { __align(this, Type, __pageBox()); }
  AlignToPageCenter(Type) { __align(this, Type || (cdrAlignHCenter | cdrAlignVCenter), __pageBox()); }
  AlignToPoint(Type, x, y) { __align(this, Type, { x0: __mm(x), x1: __mm(x), y0: __mm(y), y1: __mm(y) }); }
}
function __pageBox() {
  const p = __call('vg_page', { page: __call('vg_doc', {}).page });
  return { x0: 0, y0: 0, x1: p.w, y1: p.h };
}
function __align(geo, type, t) {
  const b = geo._b();
  let dx = 0, dy = 0;
  const h = type & 3, v = type & 12;
  if (h === cdrAlignLeft) dx = t.x0 - b.x0;
  else if (h === cdrAlignRight) dx = t.x1 - b.x1;
  else if (h === cdrAlignHCenter) dx = (t.x0 + t.x1) / 2 - (b.x0 + b.x1) / 2;
  if (v === cdrAlignTop) dy = t.y1 - b.y1;
  else if (v === cdrAlignBottom) dy = t.y0 - b.y0;
  else if (v === cdrAlignVCenter) dy = (t.y0 + t.y1) / 2 - (b.y0 + b.y1) / 2;
  if (dx || dy) __xf(geo._ids(), __M.move(dx, dy));
}

// ----- Shape --------------------------------------------------------------

function __typeOf(info) {
  switch (info.kind) {
    case 'rect': return cdrRectangleShape;
    case 'ellipse': return cdrEllipseShape;
    case 'polygon': return cdrPolygonShape;
    case 'path': return cdrCurveShape;
    case 'text': return cdrTextShape;
    case 'group': return cdrGroupShape;
    case 'bitmap': return cdrBitmapShape;
    case 'symbol': return cdrSymbolShape;
    case 'table': return cdrCustomShape;
    case 'clipframe': return __typeOf(__call('vg_info', { id: info.frame }));
    default: return cdrNoShape;
  }
}
class Shape extends __Geo {
  constructor(id) { super(); this.id = id; }
  _ids() { return [this.id]; }
  _info() { return __call('vg_info', { id: this.id }); }
  get Type() { return __typeOf(this._info()); }
  get Name() { return this._info().name; }
  set Name(n) { __call('set_name', { id: this.id, name: String(n) }); }
  get StaticID() { return this.id; }
  get Selected() { return __call('selection', {}).ids.includes(this.id); }
  set Selected(v) { if (v) this.AddToSelection(); else this.RemoveFromSelection(); }
  IsSame(Shape) { return Shape && Shape.id === this.id; }
  get IsValid() { return __call('vg_exists', { id: this.id }).yes; }
  get RotationAngle() { const a = this._info().rotation; return ((a % 360) + 360) % 360; }
  set RotationAngle(a) { this.Rotate(a - this.RotationAngle); }
  _rotCenter() { const c = __vg.rot.get(this.id); return c || this._center(); }
  get RotationCenterX() { return __un(this._rotCenter()[0]); }
  get RotationCenterY() { return __un(this._rotCenter()[1]); }
  set RotationCenterX(v) { __vg.rot.set(this.id, [__mm(v), this._rotCenter()[1]]); }
  set RotationCenterY(v) { __vg.rot.set(this.id, [this._rotCenter()[0], __mm(v)]); }
  Duplicate(OffsetX, OffsetY) { return new Shape(__call('vg_duplicate', { ids: [this.id], dx: __mm(OffsetX || 0), dy: __mm(OffsetY || 0) }).ids[0]); }
  Clone(OffsetX, OffsetY) { return this.Duplicate(OffsetX, OffsetY); }
  BreakApart() { __call('vg_break_apart', { id: this.id }); }
  BreakApartEx() { return new ShapeRange(__call('vg_break_apart', { id: this.id }).ids); }
  _bool(op, TargetShape, LeaveSource, LeaveTarget) {
    const id = __call('vg_boolean', { op: op, target: __ids(TargetShape)[0], others: [this.id],
      leave_source: !!LeaveSource, leave_target: !!LeaveTarget }).id;
    return id ? new Shape(id) : null;
  }
  Weld(TargetShape, LeaveSource, LeaveTarget) { return this._bool('weld', TargetShape, LeaveSource, LeaveTarget); }
  Trim(TargetShape, LeaveSource, LeaveTarget) { return this._bool('trim', TargetShape, LeaveSource, LeaveTarget); }
  Intersect(TargetShape, LeaveSource, LeaveTarget) { return this._bool('intersect', TargetShape, LeaveSource, LeaveTarget); }
  Ungroup() { __call('vg_ungroup', { id: this.id, all: false }); }
  UngroupEx() { return new ShapeRange(__call('vg_ungroup', { id: this.id, all: false }).ids); }
  UngroupAll() { __call('vg_ungroup', { id: this.id, all: true }); }
  UngroupAllEx() { return new ShapeRange(__call('vg_ungroup', { id: this.id, all: true }).ids); }
  get Shapes() { const id = this.id; return new Shapes(() => __call('vg_info', { id: id }).children); }
  get Layer() { const l = this._info().layer; return l ? new Layer(l) : null; }
  get Page() { return new Page(this._info().page); }
  get ParentGroup() {
    const p = this._info().parent;
    return p && __call('vg_info', { id: p }).kind === 'group' ? new Shape(p) : null;
  }
  get ClipFrame() {
    const info = this._info();
    if (info.kind !== 'clipframe') return null;
    const id = this.id;
    return { get Shapes() { return new Shapes(() => __call('vg_info', { id: id }).contents); },
      get Parent() { return new Shape(id); },
      // Editing the contents in place is a window mode; macros reach the
      // contents through Shapes either way.
      EnterEditMode() { }, LeaveEditMode() { } };
  }
  get ClipFrameParent() {
    const p = this._info().parent;
    if (!p) return null;
    const pi = __call('vg_info', { id: p });
    return pi.kind === 'clipframe' && pi.contents.includes(this.id) ? new Shape(p) : null;
  }
  get Curve() { return new __ShapeCurve(this.id); }
  get Text() { return this._info().kind === 'text' ? new __Text(this.id) : null; }
  get Rectangle() { return this._info().kind === 'rect' ? new __Rect(this.id) : null; }
  get Ellipse() { return this._info().kind === 'ellipse' ? new __Ellipse(this.id) : null; }
  get Polygon() { return this._info().kind === 'polygon' ? new __Polygon(this.id) : null; }
  get Bitmap() {
    if (this._info().kind !== 'bitmap') return null;
    const k = __call('vg_kind', { id: this.id });
    return { SizeWidth: k.width_px, SizeHeight: k.height_px };
  }
  get Effects() {
    const list = this._info().effects;
    return { Count: list.length, Item: (i) => ({ Type: list[i - 1], Index: i }),
      [Symbol.iterator]: () => __iter(list.map((t, i) => ({ Type: t, Index: i + 1 }))) };
  }
  get ObjectData() { return new __ObjectData(this.id); }
  get Properties() { return new __ObjectData(this.id); }
}

class __ObjectData {
  constructor(id) { this.id = id; }
  get _d() { return __call('vg_info', { id: this.id }).data; }
  get Count() { return this._d.length; }
  Item(IndexOrName) {
    const d = this._d, id = this.id;
    const i = typeof IndexOrName === 'number' ? IndexOrName - 1 : d.findIndex(p => p[0] === IndexOrName);
    const name = i >= 0 && i < d.length ? d[i][0] : String(IndexOrName);
    return {
      Name: name,
      get Value() { const e = __call('vg_info', { id: id }).data.find(p => p[0] === name); return e ? e[1] : ''; },
      set Value(v) {
        const data = __call('vg_info', { id: id }).data;
        const e = data.find(p => p[0] === name);
        if (e) e[1] = String(v); else data.push([name, String(v)]);
        __call('vg_set_data', { id: id, data: data });
      },
    };
  }
  Get(Name) { return this.Item(Name).Value; }
  Set(Name, Value) { this.Item(Name).Value = Value; }
  [Symbol.iterator]() { return __iter(this._d.map(p => this.Item(p[0]))); }
}

// Rectangle, ellipse and polygon views.
class __Kind {
  constructor(id) { this.id = id; }
  get k() { return __call('vg_kind', { id: this.id }); }
  put(k) { __call('vg_set_kind', { id: this.id, kind: k }); }
}
class __Rect extends __Kind {
  _radii() { const k = this.k; return k.corners ? k.corners.radii.slice() : [k.radius, k.radius, k.radius, k.radius]; }
  _half() { const r = this.k.rect; return Math.min(Math.abs(r.x1 - r.x0), Math.abs(r.y1 - r.y0)) / 2 || 1; }
  _setRadius(i, mm) {
    const k = this.k;
    const radii = k.corners ? k.corners.radii.slice() : [k.radius, k.radius, k.radius, k.radius];
    if (i < 0) radii.fill(mm); else radii[i] = mm;
    if (radii.every(r => r === radii[0])) { k.radius = radii[0]; delete k.corners; }
    else { k.corners = { radii: radii, kind: 'round' }; k.radius = 0; }
    this.put(k);
  }
  // Roundness: percent of half the shorter side. Order: top left, top
  // right, bottom left, bottom right.
  get CornerUpperLeft() { return Math.round(this._radii()[0] / this._half() * 100); }
  get CornerUpperRight() { return Math.round(this._radii()[1] / this._half() * 100); }
  get CornerLowerLeft() { return Math.round(this._radii()[2] / this._half() * 100); }
  get CornerLowerRight() { return Math.round(this._radii()[3] / this._half() * 100); }
  set CornerUpperLeft(v) { this._setRadius(0, v / 100 * this._half()); }
  set CornerUpperRight(v) { this._setRadius(1, v / 100 * this._half()); }
  set CornerLowerLeft(v) { this._setRadius(2, v / 100 * this._half()); }
  set CornerLowerRight(v) { this._setRadius(3, v / 100 * this._half()); }
  get RadiusUpperLeft() { return __un(this._radii()[0]); }
  get RadiusUpperRight() { return __un(this._radii()[1]); }
  get RadiusLowerLeft() { return __un(this._radii()[2]); }
  get RadiusLowerRight() { return __un(this._radii()[3]); }
  set RadiusUpperLeft(v) { this._setRadius(0, __mm(v)); }
  set RadiusUpperRight(v) { this._setRadius(1, __mm(v)); }
  set RadiusLowerLeft(v) { this._setRadius(2, __mm(v)); }
  set RadiusLowerRight(v) { this._setRadius(3, __mm(v)); }
  SetRoundness(Roundness) { this._setRadius(-1, Roundness / 100 * this._half()); }
  SetRadius(Radius) { this._setRadius(-1, __mm(Radius)); }
  get Width() { const r = this.k.rect; return __un(Math.abs(r.x1 - r.x0)); }
  get Height() { const r = this.k.rect; return __un(Math.abs(r.y1 - r.y0)); }
}
class __Ellipse extends __Kind {
  get StartAngle() { const a = this.k.arc; return a ? a.start_deg : 0; }
  get EndAngle() { const a = this.k.arc; return a ? a.end_deg : 360; }
  set StartAngle(v) { const k = this.k; k.arc = Object.assign({ start_deg: 0, end_deg: 360, pie: false }, k.arc || {}, { start_deg: v }); this.put(k); }
  set EndAngle(v) { const k = this.k; k.arc = Object.assign({ start_deg: 0, end_deg: 360, pie: false }, k.arc || {}, { end_deg: v }); this.put(k); }
  get Type() { const a = this.k.arc; return a ? (a.pie ? 1 : 2) : 0; }
  get Pie() { const a = this.k.arc; return !!(a && a.pie); }
  get IsCircle() { const r = this.k.rect; return Math.abs(Math.abs(r.x1 - r.x0) - Math.abs(r.y1 - r.y0)) < 1e-6; }
}
class __Polygon extends __Kind {
  get Sides() { return this.k.points; }
  set Sides(v) { const k = this.k; k.points = Math.max(3, Math.round(v)); this.put(k); }
  get Type() { return this.k.sharpness > 0 ? 1 : 0; }
  get Sharpness() { return Math.round(this.k.sharpness * 100); }
  set Sharpness(v) { const k = this.k; k.sharpness = v / 100; this.put(k); }
}

// ----- Curves -------------------------------------------------------------

const __nodeType = { cusp: cdrCuspNode, smooth: cdrSmoothNode, symmetrical: cdrSymmetricalNode };
class __Node {
  constructor(id, index) { this.id = id; this.index = index; }
  get _n() { return __call('curve_nodes', { id: this.id }).nodes.find(n => n.index === this.index); }
  get PositionX() { return __un(this._n.x); }
  get PositionY() { return __un(this._n.y); }
  set PositionX(v) { this.SetPosition(v, this.PositionY); }
  set PositionY(v) { this.SetPosition(this.PositionX, v); }
  GetPosition() { return [this.PositionX, this.PositionY]; }
  SetPosition(PositionX, PositionY) { __call('node_move', { id: this.id, index: this.index, x: __mm(PositionX), y: __mm(PositionY) }); }
  Move(DeltaX, DeltaY) { __call('node_move', { id: this.id, index: this.index, dx: __mm(DeltaX || 0), dy: __mm(DeltaY || 0) }); }
  get Type() { return __nodeType[this._n.type]; }
  set Type(t) { __call('node_type', { id: this.id, index: this.index, type: ['cusp', 'smooth', 'symmetrical'][t] || 'cusp' }); }
  get IsEnding() { return this._n.ending; }
  get SubPathIndex() { return this._n.subpath + 1; }
  get Index() { const all = __call('curve_nodes', { id: this.id }).nodes; return all.findIndex(n => n.index === this.index) + 1; }
  Delete() { __call('node_delete', { id: this.id, index: this.index }); }
  BreakApart() { }
}
class __Nodes {
  constructor(id, sub) { this.id = id; this.sub = sub; }
  get _list() { const all = __call('curve_nodes', { id: this.id }).nodes; return this.sub === null ? all : all.filter(n => n.subpath === this.sub); }
  get Count() { return this._list.length; }
  Item(Index) { const n = this._list[Index - 1]; return n ? new __Node(this.id, n.index) : null; }
  get First() { return this.Item(1); }
  get Last() { return this.Item(this.Count); }
  All() { return this._list.map(n => new __Node(this.id, n.index)); }
  [Symbol.iterator]() { return __iter(this.All()); }
  forEach(f) { this.All().forEach(f); }
}
class __SubPath {
  constructor(id, i) { this.id = id; this.i = i; }
  get Nodes() { return new __Nodes(this.id, this.i); }
  get Closed() { return __call('curve_info', { id: this.id }).subpaths[this.i].closed; }
  get Index() { return this.i + 1; }
  get StartNode() { return this.Nodes.First; }
  get EndNode() { return this.Nodes.Last; }
}
class __Segment {
  constructor(start, end, index) { this.StartNode = start; this.EndNode = end; this.Index = index; }
}
class __ShapeCurve {
  constructor(id) { this.id = id; }
  get Nodes() { return new __Nodes(this.id, null); }
  get SubPaths() {
    const id = this.id;
    const n = () => __call('curve_info', { id: id }).subpaths.length;
    return { get Count() { return n(); }, Item: (i) => (i >= 1 && i <= n() ? new __SubPath(id, i - 1) : null),
      [Symbol.iterator]: () => __iter(Array.from({ length: n() }, (_, i) => new __SubPath(id, i))) };
  }
  get Segments() {
    const info = __call('curve_info', { id: this.id });
    const list = [];
    info.subpaths.forEach((sp, s) => {
      const nodes = info.nodes.filter(n => n.subpath === s);
      for (let i = 0; i + 1 < nodes.length; i++) list.push([nodes[i], nodes[i + 1]]);
      if (sp.closed && nodes.length > 1) list.push([nodes[nodes.length - 1], nodes[0]]);
    });
    const id = this.id;
    const segs = list.map((p, i) => new __Segment(new __Node(id, p[0].index), new __Node(id, p[1].index), i + 1));
    return { Count: segs.length, Item: (i) => segs[i - 1] || null, [Symbol.iterator]: () => __iter(segs) };
  }
  get Closed() { return __call('curve_info', { id: this.id }).subpaths.every(p => p.closed); }
  get Length() { return __un(__call('vg_length', { id: this.id }).length); }
}
// A curve built by a macro (CreateCurve) before it becomes a shape.
class __BuildSubPath {
  constructor(x, y) { this.p = ['M ' + __mm(x) + ' ' + __mm(y)]; this.Closed = false; }
  AppendLineSegment(x, y) { this.p.push('L ' + __mm(x) + ' ' + __mm(y)); }
  AppendCurveSegment2(x, y, StartingControlPointX, StartingControlPointY, EndingControlPointX, EndingControlPointY) {
    this.p.push('C ' + [StartingControlPointX, StartingControlPointY, EndingControlPointX, EndingControlPointY, x, y].map(__mm).join(' '));
  }
  _d() { return this.p.join(' ') + (this.Closed ? ' Z' : ''); }
}
class Curve {
  constructor() { this.subs = []; }
  CreateSubPath(x, y) { const s = new __BuildSubPath(x, y); this.subs.push(s); return s; }
  get SubPaths() { const subs = this.subs; return { Count: subs.length, Item: (i) => subs[i - 1] }; }
  get Closed() { return this.subs.length > 0 && this.subs.every(s => s.Closed); }
  set Closed(v) { this.subs.forEach(s => { s.Closed = !!v; }); }
  _d() { return this.subs.map(s => s._d()).join(' '); }
}

// ----- Text ---------------------------------------------------------------

const __align2cdr = { none: cdrNoAlignment, left: cdrLeftAlignment, right: cdrRightAlignment,
  center: cdrCenterAlignment, justify: cdrFullJustifyAlignment, force_justify: cdrForceJustifyAlignment };
const __cdr2align = ['none', 'left', 'right', 'center', 'justify', 'force_justify'];
class __Story {
  constructor(id) { this.id = id; }
  get k() { return __call('vg_kind', { id: this.id }); }
  put(k) { __call('vg_set_kind', { id: this.id, kind: k }); }
  _spans(f) { const k = this.k; k.spans.forEach(f); this.put(k); }
  get Text() { return this.k.spans.map(s => s.text).join(''); }
  set Text(t) { __call('set_text', { id: this.id, text: String(t) }); }
  get Font() { const s = this.k.spans[0]; return s ? s.font_family : ''; }
  set Font(f) { this._spans(s => { s.font_family = String(f); }); }
  get Size() { const s = this.k.spans[0]; return s ? s.size_pt : 0; }
  set Size(v) { this._spans(s => { s.size_pt = Number(v); }); }
  get Bold() { const s = this.k.spans[0]; return !!(s && s.bold); }
  set Bold(v) { this._spans(s => { s.bold = !!v; }); }
  get Italic() { const s = this.k.spans[0]; return !!(s && s.italic); }
  set Italic(v) { this._spans(s => { s.italic = !!v; }); }
  get Underline() { const s = this.k.spans[0]; return s && s.underline ? cdrSingleThinFontLine : cdrNoFontLine; }
  set Underline(v) { this._spans(s => { s.underline = !!v; }); }
  get Alignment() { const a = __align2cdr[this.k.align]; return a === undefined ? cdrLeftAlignment : a; }
  set Alignment(v) { const k = this.k; k.align = __cdr2align[v] || 'left'; this.put(k); }
  get Length() { return this.Text.length; }
  get Characters() { const t = this.Text; return { Count: t.length }; }
  get Words() { const t = this.Text.split(/\s+/).filter(Boolean); return { Count: t.length }; }
}
class __Text {
  constructor(id) { this.id = id; }
  get Story() { return new __Story(this.id); }
  get Type() { return __call('vg_kind', { id: this.id }).frame ? cdrParagraphText : cdrArtisticText; }
  get IsArtisticText() { return this.Type === cdrArtisticText; }
  Replace(OldText, NewText, CaseSensitive) {
    const st = this.Story, t = st.Text;
    const esc = String(OldText).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const out = t.replace(new RegExp(esc, CaseSensitive ? 'g' : 'gi'), String(NewText));
    if (out !== t) st.Text = out;
    return out !== t;
  }
}

// ----- Collections and ranges ---------------------------------------------

function __itemOf(list, IndexOrName) {
  if (typeof IndexOrName === 'number') {
    const id = list[IndexOrName - 1];
    if (id === undefined) throw new Error('The index is out of range: ' + IndexOrName);
    return new Shape(id);
  }
  const id = list.find(i => __call('vg_info', { id: i }).name === String(IndexOrName));
  if (id === undefined) throw new Error('No object named ' + IndexOrName);
  return new Shape(id);
}
function __matches(id, Name, Type, query) {
  const info = __call('vg_info', { id: id });
  if (Name !== undefined && Name !== '' && info.name !== String(Name)) return false;
  if (Type !== undefined && Type !== 0 && Type !== cdrNoShape && __typeOf(info) !== Type) return false;
  if (query) return __query(info, query);
  return true;
}
const __typeNames = { rectangle: cdrRectangleShape, ellipse: cdrEllipseShape, curve: cdrCurveShape,
  polygon: cdrPolygonShape, bitmap: cdrBitmapShape, text: cdrTextShape, group: cdrGroupShape,
  symbol: cdrSymbolShape };
// Queries: comparisons of @name, @type and @fill.type joined by "and" /
// "or", e.g. "@name = 'Logo' and @type = 'rectangle'".
function __query(info, q) {
  return String(q).split(/\s+or\s+/i).some(part => part.split(/\s+and\s+/i).every(t => {
    const m = /^\s*@([\w.]+)\s*(=|<>|!=)\s*'([^']*)'\s*$/.exec(t);
    if (!m) return false;
    let v;
    switch (m[1].toLowerCase()) {
      case 'name': v = info.name; break;
      case 'type': v = Object.keys(__typeNames).find(k => __typeNames[k] === __typeOf(info)) || ''; break;
      case 'fill.type': v = { none: 'none', solid: 'uniform', fountain: 'fountain', pattern: 'pattern', texture: 'texture', mesh: 'mesh' }[info.fill.fill]; break;
      default: return false;
    }
    const eq = String(v).toLowerCase() === m[3].toLowerCase();
    return m[2] === '=' ? eq : !eq;
  }));
}
function __find(rootIds, Name, Type, Recursive, Query) {
  const out = [];
  const walk = (list) => {
    for (const id of list) {
      if (__matches(id, Name, Type, Query)) out.push(id);
      if (Recursive) {
        const info = __call('vg_info', { id: id });
        walk(info.children.concat(info.frame ? [info.frame] : [], info.contents));
      }
    }
  };
  walk(rootIds);
  return out;
}
class ShapeRange extends __Geo {
  constructor(ids) { super(); this.ids = (ids || []).slice(); }
  _ids() { return this.ids.filter(id => __call('vg_exists', { id: id }).yes); }
  get Count() { return this.ids.length; }
  Item(IndexOrName) { return __itemOf(this.ids, IndexOrName); }
  get Shapes() { return new Shapes(() => this.ids); }
  get FirstShape() { return this.ids.length ? new Shape(this.ids[0]) : null; }
  get LastShape() { return this.ids.length ? new Shape(this.ids[this.ids.length - 1]) : null; }
  Add(Shape) { for (const id of __ids(Shape)) if (!this.ids.includes(id)) this.ids.push(id); return this.ids.length; }
  AddRange(ShapeRange) { this.Add(ShapeRange); }
  Remove(Index) { this.ids.splice(Index - 1, 1); }
  RemoveAll() { this.ids = []; }
  RemoveRange(ShapeRange) { const drop = __ids(ShapeRange); this.ids = this.ids.filter(i => !drop.includes(i)); }
  IndexOf(Shape) { return this.ids.indexOf(__ids(Shape)[0]) + 1; }
  Exists(Shape) { return this.ids.includes(__ids(Shape)[0]); }
  Range(IndexArray) { return new ShapeRange([].concat(IndexArray).map(i => this.ids[i - 1]).filter(i => i !== undefined)); }
  ReverseRange() { return new ShapeRange(this.ids.slice().reverse()); }
  Reverse() { this.ids.reverse(); }
  // Sort: "@shape1.Left < @shape2.Left" style expressions on Left, Right,
  // Top, Bottom, CenterX, CenterY, Width, Height and Name.
  Sort(CompareExpression, StartIndex, EndIndex) {
    const m = /@shape1\.(\w+)\s*([<>])\s*@shape2\.(\w+)/i.exec(String(CompareExpression || ''));
    const key = (m ? m[1] : 'Left').toLowerCase();
    const desc = m && m[2] === '>';
    const val = (id) => {
      if (key === 'name') return __call('vg_info', { id: id }).name;
      const b = __call('vg_bounds', { ids: [id] });
      return { left: b.x0, right: b.x1, top: b.y1, bottom: b.y0, centerx: (b.x0 + b.x1) / 2,
        centery: (b.y0 + b.y1) / 2, width: b.x1 - b.x0, height: b.y1 - b.y0 }[key] || 0;
    };
    const s = (StartIndex || 1) - 1, e = EndIndex || this.ids.length;
    const part = this.ids.slice(s, e).map(id => [val(id), id]);
    part.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0) * (desc ? -1 : 1));
    this.ids.splice(s, part.length, ...part.map(p => p[1]));
  }
  Group() { return new Shape(__call('vg_group', { ids: this._ids() }).id); }
  Combine() { return new Shape(__call('vg_combine', { ids: this._ids() }).id); }
  Ungroup() { this._ids().forEach(id => __call('vg_ungroup', { id: id, all: false })); }
  UngroupAll() { this._ids().forEach(id => __call('vg_ungroup', { id: id, all: true })); }
  UngroupAllEx() { const out = []; this._ids().forEach(id => out.push(...__call('vg_ungroup', { id: id, all: true }).ids)); return new ShapeRange(out); }
  UngroupEx() { const out = []; this._ids().forEach(id => out.push(...__call('vg_ungroup', { id: id, all: false }).ids)); return new ShapeRange(out); }
  BreakApart() { this._ids().forEach(id => __call('vg_break_apart', { id: id })); }
  Duplicate(OffsetX, OffsetY) { return new ShapeRange(__call('vg_duplicate', { ids: this._ids(), dx: __mm(OffsetX || 0), dy: __mm(OffsetY || 0) }).ids); }
  Clone(OffsetX, OffsetY) { return this.Duplicate(OffsetX, OffsetY); }
  FindShapes(Name, Type, Recursive, Query) { return new ShapeRange(__find(this._ids(), Name, Type, Recursive === undefined ? true : Recursive, Query)); }
  FindShape(Name, Type, StaticID, Recursive, Query) {
    if (StaticID) return this.ids.includes(StaticID) ? new Shape(StaticID) : null;
    const r = __find(this._ids(), Name, Type, Recursive === undefined ? true : Recursive, Query);
    return r.length ? new Shape(r[0]) : null;
  }
  // MethodH and MethodV take cdrAlignDistributeH / cdrAlignDistributeV:
  // 1 to 3 align (right or top, left or bottom, centre) to the last shape
  // of the range, the page edge or centre, or a point (AlignTo); 4 to 6
  // spread the same lines evenly and 7 makes the gaps equal, across the
  // range or the page or DistributeRect (DistributeArea).
  AlignAndDistribute(MethodH, MethodV, AlignTo, DistributeArea, UseOutline, TextAlignOrigin, PointX, PointY, DistributeRect) {
    const ids = this._ids();
    if (!ids.length) return;
    const to = AlignTo === undefined ? cdrAlignShapesToLastSelected : AlignTo;
    if ((MethodH || MethodV) && to === cdrAlignShapesToGrid) __no('ShapeRange.AlignAndDistribute', 'there is no grid to align to');
    const box = (id) => __call('vg_bounds', { ids: [id] });
    const pick = (b, m, lo, hi) => (m === 1 || m === 4 ? b[hi] : m === 2 || m === 5 ? b[lo] : (b[lo] + b[hi]) / 2);
    let area = null;
    if (DistributeArea === cdrDistributeToPage) area = __pageBox();
    else if (DistributeArea === cdrDistributeToRect && DistributeRect) {
      const r = DistributeRect;
      area = { x0: __mm(r.x), y0: __mm(r.y), x1: __mm(r.x + r.Width), y1: __mm(r.y + r.Height) };
    }
    const axis = (m, isX) => {
      if (!m) return;
      const lo = isX ? 'x0' : 'y0', hi = isX ? 'x1' : 'y1';
      const move = (id, d) => { if (d) __xf([id], isX ? __M.move(d, 0) : __M.move(0, d)); };
      if (m <= 3) {
        let target;
        if (to === cdrAlignShapesToEdgeOfPage) target = pick(__pageBox(), m, lo, hi);
        else if (to === cdrAlignShapesToCenterOfPage) { const p = __pageBox(); target = (p[lo] + p[hi]) / 2; }
        else if (to === cdrAlignShapesToPoint) target = __mm(isX ? PointX : PointY);
        else { if (ids.length < 2) return; target = pick(box(ids[ids.length - 1]), m, lo, hi); }
        for (const id of ids) move(id, target - pick(box(id), m, lo, hi));
        return;
      }
      if (ids.length < 2) return;
      const items = ids.map(id => [id, box(id)]);
      if (m === 7) {
        items.sort((a, b) => a[1][lo] - b[1][lo]);
        const total = items.reduce((t, it) => t + it[1][hi] - it[1][lo], 0);
        const from = area ? area[lo] : items[0][1][lo];
        const to2 = area ? area[hi] : Math.max(...items.map(it => it[1][hi]));
        const gap = (to2 - from - total) / (items.length - 1);
        let at = from;
        for (const [id, b] of items) { move(id, at - b[lo]); at += b[hi] - b[lo] + gap; }
        return;
      }
      items.sort((a, b) => pick(a[1], m, lo, hi) - pick(b[1], m, lo, hi));
      const b0 = items[0][1], bn = items[items.length - 1][1];
      const first = area ? area[lo] + pick(b0, m, lo, hi) - b0[lo] : pick(b0, m, lo, hi);
      const last = area ? area[hi] - (bn[hi] - pick(bn, m, lo, hi)) : pick(bn, m, lo, hi);
      items.forEach(([id, b], i) => move(id, first + (last - first) * i / (items.length - 1) - pick(b, m, lo, hi)));
    };
    axis(MethodH, true);
    axis(MethodV, false);
  }
  DistributeHorizontally() { this.AlignAndDistribute(cdrAlignDistributeHDistributeCenter, cdrAlignDistributeVNone); }
  DistributeVertically() { this.AlignAndDistribute(cdrAlignDistributeHNone, cdrAlignDistributeVDistributeCenter); }
  [Symbol.iterator]() { return __iter(this.ids.map(id => new Shape(id))); }
  forEach(f) { this.ids.map(id => new Shape(id)).forEach(f); }
}
// A live collection of shapes (a layer's, a page's, a group's).
class Shapes {
  constructor(source) { this.src = source; }
  get _list() { return this.src(); }
  get Count() { return this._list.length; }
  Item(IndexOrName) { return __itemOf(this._list, IndexOrName); }
  All() { return new ShapeRange(this._list); }
  Range(IndexArray) { const l = this._list; return new ShapeRange([].concat(IndexArray).map(i => l[i - 1]).filter(i => i !== undefined)); }
  AllExcluding(IndexArray) { const drop = [].concat(IndexArray); return new ShapeRange(this._list.filter((_, i) => !drop.includes(i + 1))); }
  FindShapes(Name, Type, Recursive, Query) { return new ShapeRange(__find(this._list, Name, Type, Recursive === undefined ? true : Recursive, Query)); }
  FindShape(Name, Type, StaticID, Recursive, Query) {
    if (StaticID) return __call('vg_exists', { id: StaticID }).yes ? new Shape(StaticID) : null;
    const r = __find(this._list, Name, Type, Recursive === undefined ? true : Recursive, Query);
    return r.length ? new Shape(r[0]) : null;
  }
  [Symbol.iterator]() { return __iter(this._list.map(id => new Shape(id))); }
  forEach(f) { this._list.map(id => new Shape(id)).forEach(f); }
}
// The selection as one object (ActiveSelection, Document.Selection()).
class SelectionShape extends __Geo {
  _ids() { return __call('selection', {}).ids; }
  get Type() { return cdrSelectionShape; }
  get Shapes() { return new Shapes(() => __call('selection', {}).ids); }
  get Name() { return ''; }
  Duplicate(OffsetX, OffsetY) { return new ShapeRange(__call('vg_duplicate', { ids: this._ids(), dx: __mm(OffsetX || 0), dy: __mm(OffsetY || 0) }).ids); }
  Group() { return new Shape(__call('vg_group', { ids: this._ids() }).id); }
  Combine() { return new Shape(__call('vg_combine', { ids: this._ids() }).id); }
  UngroupAll() { this._ids().forEach(id => __call('vg_ungroup', { id: id, all: true })); }
  UngroupAllEx() { const out = []; this._ids().forEach(id => out.push(...__call('vg_ungroup', { id: id, all: true }).ids)); return new ShapeRange(out); }
  UngroupEx() { const out = []; this._ids().forEach(id => out.push(...__call('vg_ungroup', { id: id, all: false }).ids)); return new ShapeRange(out); }
  Ungroup() { this._ids().forEach(id => __call('vg_ungroup', { id: id, all: false })); }
}

// ----- Layers, pages, documents --------------------------------------------

function __stroke() { return __defaultStroke(); }
class Layer {
  constructor(id) { this.id = id; }
  get _l() { return __call('vg_layer', { layer: this.id }); }
  get Shapes() { const id = this.id; return new Shapes(() => __call('shapes', { layer: id }).ids); }
  get Name() { return this._l.name; }
  set Name(n) { __call('set_layer_name', { layer: this.id, name: String(n) }); }
  get Visible() { return this._l.visible; }
  set Visible(v) { __call('set_layer_visible', { layer: this.id, visible: !!v }); }
  get Printable() { return this._l.printable; }
  set Printable(v) { __call('vg_layer_set', { layer: this.id, printable: !!v }); }
  get Editable() { return !this._l.locked; }
  set Editable(v) { __call('vg_layer_set', { layer: this.id, locked: !v }); }
  get Master() { return this._l.master; }
  get IsSpecialLayer() { return this._l.master; }
  get Page() { return new Page(this._l.page); }
  get Index() { const p = __call('vg_page', { page: this._l.page }); return p.layers.indexOf(this.id) + 1; }
  Activate() { __call('activate_layer', { layer: this.id }); }
  Delete() { __call('vg_delete_layer', { layer: this.id }); }
  _add(kind, fill, stroke) {
    const a = { layer: this.id, kind: kind };
    if (fill !== undefined) a.fill = fill;
    if (stroke !== undefined) a.stroke = stroke;
    return new Shape(__call('vg_add_shape', a).id);
  }
  _rect(x0, y0, x1, y1) {
    const r = { x0: Math.min(x0, x1), y0: Math.min(y0, y1), x1: Math.max(x0, x1), y1: Math.max(y0, y1) };
    return r;
  }
  CreateRectangle(Left, Top, Right, Bottom, CornerUL, CornerUR, CornerLR, CornerLL) {
    const r = this._rect(__mm(Left), __mm(Bottom), __mm(Right), __mm(Top));
    const s = this._add({ type: 'rect', rect: r, radius: 0 }, { fill: 'none' }, __stroke());
    if (CornerUL || CornerUR || CornerLR || CornerLL) {
      const v = s.Rectangle;
      if (CornerUL === CornerUR && CornerUR === CornerLR && CornerLR === CornerLL) v.SetRoundness(CornerUL);
      else { v.CornerUpperLeft = CornerUL || 0; v.CornerUpperRight = CornerUR || 0; v.CornerLowerRight = CornerLR || 0; v.CornerLowerLeft = CornerLL || 0; }
    }
    return s;
  }
  CreateRectangle2(x, y, Width, Height, RadiusUL, RadiusUR, RadiusLR, RadiusLL) {
    const s = this.CreateRectangle(x, y + Height, x + Width, y);
    if (RadiusUL || RadiusUR || RadiusLR || RadiusLL) {
      const v = s.Rectangle;
      v.RadiusUpperLeft = RadiusUL || 0; v.RadiusUpperRight = RadiusUR || 0;
      v.RadiusLowerRight = RadiusLR || 0; v.RadiusLowerLeft = RadiusLL || 0;
    }
    return s;
  }
  CreateRectangleRect(Rect) { return this.CreateRectangle2(Rect.x, Rect.y, Rect.Width, Rect.Height); }
  _ellipse(r, StartAngle, EndAngle, Pie) {
    const arc = (StartAngle !== undefined && EndAngle !== undefined && StartAngle !== EndAngle)
      ? { start_deg: StartAngle, end_deg: EndAngle, pie: !!Pie } : null;
    return this._add({ type: 'ellipse', rect: r, arc: arc }, { fill: 'none' }, __stroke());
  }
  CreateEllipse(Left, Top, Right, Bottom, StartAngle, EndAngle, Pie) {
    return this._ellipse(this._rect(__mm(Left), __mm(Bottom), __mm(Right), __mm(Top)), StartAngle, EndAngle, Pie);
  }
  CreateEllipse2(CenterX, CenterY, Radius1, Radius2, StartAngle, EndAngle, Pie) {
    const cx = __mm(CenterX), cy = __mm(CenterY), r1 = __mm(Radius1), r2 = Radius2 ? __mm(Radius2) : r1;
    return this._ellipse(this._rect(cx - r1, cy - r2, cx + r1, cy + r2), StartAngle, EndAngle, Pie);
  }
  CreatePolygon(Left, Top, Right, Bottom, Sides, SubPaths, Complexity, Star, StarComplexity, Sharpness) {
    const r = this._rect(__mm(Left), __mm(Bottom), __mm(Right), __mm(Top));
    const sharp = Star ? (Sharpness === undefined ? 0.5 : Sharpness / 100) : 0;
    return this._add({ type: 'polygon', rect: r, points: Math.max(3, Sides || 5), sharpness: sharp }, { fill: 'none' }, __stroke());
  }
  CreatePolygon2(CenterX, CenterY, Radius, Sides) {
    return this.CreatePolygon(CenterX - Radius, CenterY + Radius, CenterX + Radius, CenterY - Radius, Sides);
  }
  CreateCurve(Source) {
    const d = Source && typeof Source._d === 'function' ? Source._d() : '';
    const s = new Shape(__call('create_path', { layer: this.id, d: d }).id);
    __call('vg_set_stroke', { ids: [s.id], stroke: __stroke() });
    return s;
  }
  CreateLineSegment(StartX, StartY, EndX, EndY) {
    return new Shape(__call('create_curve', { layer: this.id, points: [[__mm(StartX), __mm(StartY)], [__mm(EndX), __mm(EndY)]], closed: false }).id);
  }
  CreateCurveSegment(StartX, StartY, EndX, EndY, StartingControlPointLength, StartingControlPointAngle, EndingControlPointLength, EndingControlPointAngle) {
    const sx = __mm(StartX), sy = __mm(StartY), ex = __mm(EndX), ey = __mm(EndY);
    const pol = (x, y, len, ang) => [x + __mm(len || 0) * Math.cos((ang || 0) * Math.PI / 180), y + __mm(len || 0) * Math.sin((ang || 0) * Math.PI / 180)];
    const c1 = pol(sx, sy, StartingControlPointLength, StartingControlPointAngle);
    const c2 = pol(ex, ey, EndingControlPointLength, EndingControlPointAngle);
    const s = new Shape(__call('create_path', { layer: this.id, d: 'M ' + sx + ' ' + sy + ' C ' + [c1[0], c1[1], c2[0], c2[1], ex, ey].join(' ') }).id);
    __call('vg_set_stroke', { ids: [s.id], stroke: __stroke() });
    return s;
  }
  CreateArtisticText(Left, Bottom, Text, LanguageID, CharSet, Font, Size, Bold, Italic, Underline, Alignment) {
    const s = new Shape(__call('create_text', { layer: this.id, x: __mm(Left), y: __mm(Bottom), text: String(Text === undefined ? '' : Text), font: Font || '', size: Size || 24, frame: null }).id);
    const st = s.Text.Story;
    if (Bold) st.Bold = true;
    if (Italic) st.Italic = true;
    if (Underline) st.Underline = Underline;
    if (Alignment) st.Alignment = Alignment;
    return s;
  }
  CreateParagraphText(Left, Top, Right, Bottom, Text, LanguageID, CharSet, Font, Size, Bold, Italic, Underline, Alignment) {
    const x0 = Math.min(__mm(Left), __mm(Right)), y0 = Math.min(__mm(Top), __mm(Bottom));
    const w = Math.abs(__mm(Right) - __mm(Left)), h = Math.abs(__mm(Top) - __mm(Bottom));
    const s = new Shape(__call('create_text', { layer: this.id, x: x0, y: y0, text: String(Text === undefined ? '' : Text), font: Font || '', size: Size || 12, frame: [w, h] }).id);
    const st = s.Text.Story;
    if (Bold) st.Bold = true;
    if (Italic) st.Italic = true;
    if (Underline) st.Underline = Underline;
    if (Alignment) st.Alignment = Alignment;
    return s;
  }
  Import(FileName) { return new Shape(__call('import_bitmap', { layer: this.id, path: String(FileName) }).id); }
  ImportBitmap(FileName) { return this.Import(FileName); }
  CreateBitmap(FileName) { return this.Import(FileName); }
  Paste() { throw new Error('Paste is not available to macros'); }
}
class Layers {
  constructor(page) { this.page = page; }
  get _list() { return __call('vg_page', { page: this.page }).layers; }
  get Count() { return this._list.length; }
  Item(IndexOrName) {
    const l = this._list;
    if (typeof IndexOrName === 'number') { if (!l[IndexOrName - 1]) throw new Error('The index is out of range: ' + IndexOrName); return new Layer(l[IndexOrName - 1]); }
    const id = l.find(i => __call('vg_layer', { layer: i }).name === String(IndexOrName));
    if (id === undefined) throw new Error('No layer named ' + IndexOrName);
    return new Layer(id);
  }
  Find(LayerName) { try { return this.Item(String(LayerName)); } catch (e) { return null; } }
  [Symbol.iterator]() { return __iter(this._list.map(id => new Layer(id))); }
  forEach(f) { this._list.map(id => new Layer(id)).forEach(f); }
}
class Page {
  constructor(id) { this.id = id; }
  get _p() { return __call('vg_page', { page: this.id }); }
  get Layers() { return new Layers(this.id); }
  get AllLayers() { const p = this._p; const ids = p.layers.concat(p.master); return { Count: ids.length, Item: (i) => new Layer(ids[i - 1]), [Symbol.iterator]: () => __iter(ids.map(i => new Layer(i))) }; }
  get Shapes() { const id = this.id; return new Shapes(() => __call('page_shapes', { page: id }).ids); }
  get SelectableShapes() {
    const id = this.id;
    return new Shapes(() => __call('vg_page', { page: id }).layers
      .filter(l => { const i = __call('vg_layer', { layer: l }); return i.visible && !i.locked; })
      .flatMap(l => __call('shapes', { layer: l }).ids));
  }
  get ActiveLayer() { return new Layer(__call('active_layer', { page: this.id }).id); }
  get Name() { return this._p.name; }
  set Name(n) { __call('vg_rename_page', { page: this.id, name: String(n) }); }
  get Index() { return this._p.index; }
  get SizeWidth() { return __un(this._p.w); }
  get SizeHeight() { return __un(this._p.h); }
  set SizeWidth(v) { __call('set_page_size', { page: this.id, w: __mm(v), h: this._p.h }); }
  set SizeHeight(v) { __call('set_page_size', { page: this.id, w: this._p.w, h: __mm(v) }); }
  SetSize(Width, Height) { __call('set_page_size', { page: this.id, w: __mm(Width), h: __mm(Height) }); }
  GetSize() { return [this.SizeWidth, this.SizeHeight]; }
  get Orientation() { const p = this._p; return p.w > p.h ? cdrLandscape : cdrPortrait; }
  set Orientation(v) {
    const p = this._p;
    if ((v === cdrLandscape) !== (p.w > p.h)) __call('set_page_size', { page: this.id, w: p.h, h: p.w });
  }
  get LeftX() { return 0; }
  get BottomY() { return 0; }
  get RightX() { return this.SizeWidth; }
  get TopY() { return this.SizeHeight; }
  get CenterX() { return this.SizeWidth / 2; }
  get CenterY() { return this.SizeHeight / 2; }
  Activate() { __call('activate_page', { page: this.id }); }
  Delete() { __call('vg_delete_page', { page: this.id }); }
  CreateLayer(LayerName) { return new Layer(__call('create_layer', { page: this.id, name: String(LayerName || 'Layer') }).id); }
  FindShapes(Name, Type, Recursive, Query) { return this.Shapes.FindShapes(Name, Type, Recursive, Query); }
  FindShape(Name, Type, StaticID, Recursive, Query) { return this.Shapes.FindShape(Name, Type, StaticID, Recursive, Query); }
}
class Pages {
  get _list() { return __call('vg_doc', {}).pages; }
  get Count() { return this._list.length; }
  Item(Index) { const id = this._list[Index - 1]; if (id === undefined) throw new Error('The index is out of range: ' + Index); return new Page(id); }
  get First() { return this.Item(1); }
  get Last() { return this.Item(this.Count); }
  [Symbol.iterator]() { return __iter(this._list.map(id => new Page(id))); }
  forEach(f) { this._list.map(id => new Page(id)).forEach(f); }
}
// An export waiting for Finish (ExportBitmap / ExportEx).
class ExportFilter {
  constructor(args) { this.args = args; }
  Finish() { __call('vg_export', this.args); }
  get HasDialog() { return false; }
  ShowDialog() { return true; }
  Reset() { }
}
class Document {
  get _d() { return __call('vg_doc', {}); }
  get Name() { const d = this._d; return d.file ? d.file.split(/[\\/]/).pop() : d.title; }
  get Title() { return this._d.title; }
  get FileName() { const f = this._d.file; return f ? f.split(/[\\/]/).pop() : ''; }
  get FilePath() { const f = this._d.file; return f ? f.slice(0, f.length - f.split(/[\\/]/).pop().length) : ''; }
  get FullFileName() { return this._d.file || ''; }
  get Dirty() { return this._d.changed; }
  set Dirty(v) { }
  get Unit() { return __vg.unit; }
  set Unit(v) { if (__mmPer[v] === undefined) throw new Error('Unknown unit ' + v); __vg.unit = v; }
  get ReferencePoint() { return __vg.ref; }
  set ReferencePoint(v) { __vg.ref = v; }
  get Resolution() { return this._d.dpi || 300; }
  get Pages() { return new Pages(); }
  get PageCount() { return this._d.pages.length; }
  get ActivePage() { return new Page(this._d.page); }
  get ActiveLayer() { return this.ActivePage.ActiveLayer; }
  get ActiveShape() { const s = __call('selection', {}).ids; return s.length ? new Shape(s[0]) : null; }
  Selection() { return new SelectionShape(); }
  SelectionRange() { return new ShapeRange(__call('selection', {}).ids); }
  get SelectableShapes() { return this.ActivePage.SelectableShapes; }
  ClearSelection() { __call('select', { ids: [], add: false }); }
  SelectAll() { __call('select_all', {}); }
  BeginCommandGroup(CommandName) { __vg.groupDepth++; }
  EndCommandGroup() { if (__vg.groupDepth > 0) __vg.groupDepth--; }
  Undo() { throw new Error('Undo is not available while a macro runs: its changes are one step'); }
  Redo() { throw new Error('Redo is not available while a macro runs'); }
  AddPages(NumberOfPages) { const ids = __call('vg_insert_pages', { count: NumberOfPages || 1 }).ids; return new Page(ids[0]); }
  AddPagesEx(NumberOfPages, Width, Height) {
    const p = this.AddPages(NumberOfPages);
    for (const id of __call('vg_doc', {}).pages.slice(-(NumberOfPages || 1))) __call('set_page_size', { page: id, w: __mm(Width), h: __mm(Height) });
    return p;
  }
  InsertPages(NumberOfPages, BeforePage, Page) {
    const at = (Page || 1) - (BeforePage ? 1 : 0);
    const ids = __call('vg_insert_pages', { count: NumberOfPages || 1, at: Math.max(0, at) }).ids;
    return new Page(ids[0]);
  }
  CreateCurve() { return new Curve(); }
  CreateShapeRangeFromArray(...shapes) { return new ShapeRange(shapes.flatMap(__ids)); }
  Save() { const f = this._d.file; if (!f) throw new Error('The drawing has no file yet: use SaveAs'); __call('vg_save', { path: f }); }
  SaveAs(FileName) { __call('vg_save', { path: String(FileName) }); }
  _range(Range) {
    const d = this._d;
    if (Range === cdrAllPages) return d.pages;
    return [d.page];
  }
  Export(FileName, Filter, Range, Options, PaletteOptions) {
    const fmt = __filterExt[Filter] || '';
    __call('vg_export', { path: String(FileName), format: fmt, page: this._d.page });
  }
  ExportEx(FileName, Filter, Range, Options, PaletteOptions) {
    return new ExportFilter({ path: String(FileName), format: __filterExt[Filter] || '', page: this._d.page });
  }
  ExportBitmap(FileName, Filter, Range, ImageType, Width, Height, ResolutionX, ResolutionY,
    AntiAliasingType, Dithered, Transparent, UseColorProfile, MaintainLayers, Compression, PaletteOptions, ExportArea) {
    return new ExportFilter({ path: String(FileName), format: __filterExt[Filter] || '', page: this._d.page,
      w: Width || 0, h: Height || 0, dpi: ResolutionX || 0 });
  }
  PublishToPDF(FileName) { __call('vg_export', { path: String(FileName), format: 'pdf' }); }
}

// ----- Application and globals --------------------------------------------

class __Documents {
  get Count() { return 1; }
  Item(Index) { if (Index !== 1 && Index !== undefined) throw new Error('Only the active drawing is available to a macro'); return new Document(); }
  [Symbol.iterator]() { return __iter([new Document()]); }
}
class __Window {
  get ActiveView() { return { Zoom: 100, SetViewArea() { }, ToFitPage() { }, ToFitSelection() { } }; }
  Refresh() { }
  Activate() { }
}
const Application = {
  get ActiveDocument() { return new Document(); },
  get ActivePage() { return new Document().ActivePage; },
  get ActiveLayer() { return new Document().ActiveLayer; },
  get ActiveSelection() { return new SelectionShape(); },
  get ActiveSelectionRange() { return new ShapeRange(__call('selection', {}).ids); },
  get ActiveShape() { return new Document().ActiveShape; },
  get ActiveWindow() { return new __Window(); },
  get Documents() { return new __Documents(); },
  get Version() { return '27.0'; },
  get VersionMajor() { return 27; },
  get VersionMinor() { return 0; },
  get Name() { return 'TraceDraw'; },
  get Visible() { return true; },
  get Optimization() { return __vg.optimization; },
  set Optimization(v) { __vg.optimization = !!v; },
  get EventsEnabled() { return __vg.events; },
  set EventsEnabled(v) { __vg.events = !!v; },
  Refresh() { },
  CreateColor(ColorString) { return ColorString ? __parseColor(ColorString) : new Color(); },
  CreateRGBColor(Red, Green, Blue) { const c = new Color(); c.RGBAssign(Red, Green, Blue); return c; },
  CreateCMYKColor(Cyan, Magenta, Yellow, Black) { const c = new Color(); c.CMYKAssign(Cyan, Magenta, Yellow, Black); return c; },
  CreateGrayColor(GrayValue) { const c = new Color(); c.GrayAssign(GrayValue); return c; },
  CreateHSBColor(Hue, Saturation, Brightness) { const c = new Color(); c.HSBAssign(Hue, Saturation, Brightness); return c; },
  CreateShapeRange() { return new ShapeRange([]); },
  CreateCurve(Document) { return new Curve(); },
  CreateDocument() { throw new Error('A macro works on the active drawing only'); },
  OpenDocument(FileName) { throw new Error('A macro works on the active drawing only'); },
};
// Application members are globals in macros too.
for (const name of Object.getOwnPropertyNames(Application)) {
  const d = Object.getOwnPropertyDescriptor(Application, name);
  if (d.get || d.set) {
    Object.defineProperty(globalThis, name, { get: d.get, set: d.set, configurable: true });
  } else {
    globalThis[name] = d.value;
  }
}


// ----- Members published macros use ---------------------------------------

function __no(member, why) {
  throw new Error(member + ' is not available in TraceDraw' + (why ? ': ' + why : ''));
}
function __def(cls, name, get, set) {
  Object.defineProperty(cls.prototype, name, { get: get, set: set, configurable: true });
}

// Application.Status: the progress shows in the status line.
const __status = {
  text: '', progress: 0, active: false,
  _show() { __call('vg_status', { text: this.active ? (this.text + ' ' + Math.round(this.progress) + '%').trim() : this.text }); },
  BeginProgress(Text, CanAbort) { this.text = String(Text || ''); this.progress = 0; this.active = true; this._show(); },
  get Progress() { return this.progress; },
  set Progress(v) { this.progress = Math.max(0, Math.min(100, Number(v) || 0)); this._show(); },
  EndProgress() { this.active = false; this.text = ''; this._show(); },
  SetMainText(Text) { this.text = String(Text || ''); this._show(); },
  get Aborted() { return false; },
};
const __corelScript = { RedrawScreen() { } };
const __profiles = () => __call('vg_doc', {}).profiles;
function __colorContext() {
  return {
    get RGBColorProfile() { return { Name: __profiles()[0] }; },
    get CMYKColorProfile() { return { Name: __profiles()[1] }; },
    get GrayscaleColorProfile() { return { Name: __profiles()[2] }; },
  };
}
const __colorManager = {
  get DefaultColorContext() { return __colorContext(); },
  engine: 1,
  get ColorEngine() { return this.engine; },
  set ColorEngine(v) { this.engine = v; },
};
Object.defineProperty(Application, 'Status', { get() { return __status; }, configurable: true });
Object.defineProperty(Application, 'CorelScript', { get() { return __corelScript; }, configurable: true });
Object.defineProperty(Application, 'ColorManager', { get() { return __colorManager; }, configurable: true });
Object.defineProperty(Application, 'FrameWork', { get() { return __no('Application.FrameWork', 'dockers and commands of other programs'); }, configurable: true });
Object.defineProperty(Application, 'VBE', { get() { return __no('Application.VBE'); }, configurable: true });
Object.defineProperty(Application, 'UILanguage', { get() { return cdrEnglishUS; }, configurable: true });
Application.CreateStructCreateOptions = function () { return { Units: cdrMillimeter, PageWidth: 210, PageHeight: 297, Resolution: 300 }; };
Application.CreateDocumentEx = function () { return __no('Application.CreateDocumentEx', 'a macro works on the active drawing'); };
for (const name of ['Status', 'CorelScript', 'ColorManager', 'FrameWork', 'VBE', 'UILanguage']) {
  const d = Object.getOwnPropertyDescriptor(Application, name);
  Object.defineProperty(globalThis, name, { get: d.get, configurable: true });
}
globalThis.CreateStructCreateOptions = Application.CreateStructCreateOptions;
globalThis.CreateDocumentEx = Application.CreateDocumentEx;

// The view.
const __unitOf = { Inches: cdrInch, Millimeters: cdrMillimeter, Centimeters: cdrCentimeter,
  Meters: cdrMeter, Picas: cdrPica, Points: cdrPoint, Pixels: cdrPixel, Ciceros: cdrCicero,
  Didots: cdrDidots, Feet: cdrFoot, Yards: cdrYard, Miles: cdrMile, Kilometers: cdrKilometer };
class __View {
  get OriginX() { return __un(__call('vg_doc', {}).view[0]); }
  get OriginY() { return __un(__call('vg_doc', {}).view[1]); }
  get Zoom() { return 100; }
  set Zoom(v) { }
  get Type() { return cdrEnhancedView; }
  set Type(v) { }
  SetViewPoint() { }
  SetViewArea() { }
  ToFitPage() { }
  ToFitSelection() { }
  ToFitAllObjects() { }
  Activate() { }
}
__def(__Window, 'ActiveView', function () { return new __View(); });

// Document: settings, selection, master page, rulers, views, profiles.
const __settingsStack = [];
__def(Document, 'PreserveSelection', function () { return __vg.preserve !== false; }, function (v) { __vg.preserve = !!v; });
Document.prototype.SaveSettings = function () { __settingsStack.push({ unit: __vg.unit, ref: __vg.ref, preserve: __vg.preserve }); };
Document.prototype.RestoreSettings = function () {
  const s = __settingsStack.pop();
  if (s) { __vg.unit = s.unit; __vg.ref = s.ref; __vg.preserve = s.preserve; }
};
__def(Document, 'MasterPage', function () { return new Page(0); });
__def(Document, 'Rulers', function () {
  const u = __unitOf[__call('vg_doc', {}).units] || cdrMillimeter;
  return { HUnits: u, VUnits: u };
});
__def(Document, 'Views', function () { const v = new __View(); return { Count: 1, Item: () => v, [Symbol.iterator]: () => __iter([v]) }; });
__def(Document, 'ColorContext', function () { return __colorContext(); });
Document.prototype.GetUserClick = function () { return __no('Document.GetUserClick', 'macros cannot wait for clicks'); };
Document.prototype.GetUserArea = function () { return __no('Document.GetUserArea', 'macros cannot wait for clicks'); };

// Page: unlocking, guides.
Page.prototype.UnlockAllShapes = function () { __call('vg_unlock_all', { page: this.id }); };
__def(Page, 'Guides', function () {
  const page = this.id;
  const list = () => __call('vg_guides', { page: page }).guides;
  return { get Count() { return list().length; }, Item: (i) => new __GuideShape(page, i - 1),
    [Symbol.iterator]: () => __iter(list().map((_, i) => new __GuideShape(page, i))) };
});

// Layers: the guides and grid are not layers here.
__def(Layer, 'IsGuidesLayer', function () { return false; });
__def(Layer, 'IsGridLayer', function () { return false; });
__def(Layer, 'IsDesktopLayer', function () { return false; });
Layer.prototype.Paste = function () { return __no('Layer.Paste', 'the clipboard is not open to macros'); };
Layer.prototype.CreateGuide = function (x1, y1, x2, y2) {
  const ax = __mm(x1), ay = __mm(y1), bx = __mm(x2), by = __mm(y2);
  const angle = Math.atan2(by - ay, bx - ax) * 180 / Math.PI;
  const page = this._l.page;
  return new __GuideShape(page, __call('vg_add_guide', { page: page, x: ax, y: ay, angle: angle }).index);
};
Layer.prototype.CreateGuideAngle = function (x, y, Angle) {
  const page = this._l.page;
  return new __GuideShape(page, __call('vg_add_guide', { page: page, x: __mm(x), y: __mm(y), angle: Angle || 0 }).index);
};
class __Guide {
  constructor(page, index) { this.page = page; this.index = index; }
  get _g() { return __call('vg_guides', { page: this.page }).guides[this.index]; }
  get Angle() { return this._g.angle; }
  set Angle(a) { const g = this._g; this.SetPointAndAngle(__un(g.x), __un(g.y), a); }
  get PointX() { return __un(this._g.x); }
  get PointY() { return __un(this._g.y); }
  GetPointAndAngle() { const g = this._g; return [__un(g.x), __un(g.y), g.angle]; }
  SetPointAndAngle(x, y, Angle) { __call('vg_set_guide', { page: this.page, index: this.index, x: __mm(x), y: __mm(y), angle: Angle }); }
}
class __GuideShape {
  constructor(page, index) { this.page = page; this.index = index; }
  get Type() { return cdrGuidelineShape; }
  get Guide() { return new __Guide(this.page, this.index); }
  get Name() { return 'Guideline'; }
}

// Shape: what it can wear, conversion to a bitmap, effects.
const __noFill = ['group', 'bitmap', 'symbol', 'table'];
__def(Shape, 'CanHaveFill', function () { return !__noFill.includes(this._info().kind); });
__def(Shape, 'CanHaveOutline', function () { return !__noFill.includes(this._info().kind); });
const __imageMode = { 0: 'bw', 1: 'paletted', 2: 'gray', 3: 'paletted', 4: 'rgb', 5: 'cmyk' };
function __toBitmap(ids, ImageType, Dithered, Transparent, Resolution, AntiAliasing, UseColorProfile, AlwaysOverprintBlack) {
  return new Shape(__call('vg_to_bitmap', { ids: ids, mode: __imageMode[ImageType] || 'rgb',
    dithered: !!Dithered, transparent: !!Transparent, dpi: Resolution || 300,
    anti_alias: AntiAliasing === undefined ? true : AntiAliasing !== cdrNoAntiAliasing,
    overprint: !!AlwaysOverprintBlack }).id);
}
for (const cls of [Shape, ShapeRange, SelectionShape]) {
  cls.prototype.ConvertToBitmapEx = function (Mode, Dithered, Transparent, Resolution, AntiAliasing, UseColorProfile, AlwaysOverprintBlack, OverprintBlackLimit) {
    return __toBitmap(this._ids(), Mode, Dithered, Transparent, Resolution, AntiAliasing, UseColorProfile, AlwaysOverprintBlack);
  };
  cls.prototype.ConvertToBitmap = function (BitDepth, Grayscale, Dithered, TransparentBG, Resolution, AntiAliasing) {
    return __toBitmap(this._ids(), Grayscale ? cdrGrayscaleImage : cdrRGBColorImage, Dithered, TransparentBG, Resolution, AntiAliasing);
  };
}
const __effectType = { blend: cdrBlend, extrude: cdrExtrude, envelope: cdrEnvelope, distort: cdrDistortion,
  contour: cdrContour, lens: cdrLens, perspective: cdrPerspective, bevel: cdrCustomEffect,
  shadow: cdrDropShadow, textonpath: cdrTextOnPath };
class __Effect {
  constructor(id, e) { this.id = id; this.e = e; }
  get Type() { const t = __effectType[this.e.kind]; return t === undefined ? cdrCustomEffect : t; }
  get DropShadow() {
    if (this.e.kind !== 'shadow') return null;
    const e = this.e;
    return {
      get MergeMode() { return cdrMergeNormal; },
      set MergeMode(v) { if (v !== cdrMergeNormal) __no('DropShadow.MergeMode', 'shadows are drawn in normal mode'); },
      get Color() { return new Color(e.color); },
      get Opacity() { return Math.round(e.opacity * 100); },
      get Feather() { return e.blur; },
      get OffsetX() { return __un(e.dx); },
      get OffsetY() { return __un(e.dy); },
    };
  }
  Separate() { return new ShapeRange(__call('vg_separate', { id: this.id }).ids); }
  Clear() {
    if (this.e.kind === 'shadow') { __call('vg_clear_shadow', { id: this.id }); return; }
    if (this.e.index !== undefined) __call('remove_effects', { id: this.id, index: this.e.index });
  }
}
__def(Shape, 'Effects', function () {
  const id = this.id;
  const list = __call('vg_effects', { id: id }).effects.map(e => new __Effect(id, e));
  return { Count: list.length, Item: (i) => list[i - 1], [Symbol.iterator]: () => __iter(list) };
});
Shape.prototype.Separate = function () { return new ShapeRange(__call('vg_separate', { id: this.id }).ids); };

// Fill: fountain colours, textures.
class __FountainColor {
  constructor(owner, i) { this.o = owner; this.i = i; }
  get Position() { return Math.round(this.o._f.stops[this.i].pos * 100); }
  get Color() {
    const o = this.o, i = this.i;
    return new Color(o._f.stops[i].color, (nv) => { const f = o._f; f.stops[i].color = nv; o._put(f); });
  }
}
class __Fountain {
  constructor(fill) { this.fill = fill; }
  get _f() { const f = this.fill._f; if (f.fill !== 'fountain') __no('Fill.Fountain', 'the fill is not a fountain'); return f; }
  _put(f) { this.fill._put(f); }
  get Type() { return { linear: cdrLinearFountainFill, radial: cdrRadialFountainFill, conical: cdrConicalFountainFill, square: cdrSquareFountainFill }[this._f.kind] || cdrLinearFountainFill; }
  get Angle() { return this._f.angle; }
  set Angle(v) { const f = this._f; f.angle = Number(v); this._put(f); }
  get StartColor() { return this.Colors.Item(1).Color; }
  get EndColor() { const c = this.Colors; return c.Item(c.Count).Color; }
  get Steps() { return 0; }
  set Steps(v) { __no('Fill.Fountain.Steps', 'fountains are drawn smooth'); }
  get Colors() {
    const n = this._f.stops.length, me = this;
    const list = () => Array.from({ length: n }, (_, i) => new __FountainColor(me, i));
    return { Count: n, Item: (i) => new __FountainColor(me, i - 1), [Symbol.iterator]: () => __iter(list()) };
  }
}
__def(Fill, 'Fountain', function () { return new __Fountain(this); });
__def(Fill, 'Texture', function () {
  return { get Resolution() { return __no('Fill.Texture.Resolution', 'textures are drawn at the output resolution'); } };
});

// Outline: scaling with the object, miter limit.
__def(Outline, 'ScaleWithShape', function () { const s = this._s; return !!(s && s.scale_with_object); },
  function (v) { const s = this._s || __defaultStroke(); s.scale_with_object = !!v; this._put(s); });
// The renderer's miter ratio of 4, as the angle below which corners are
// beveled.
__def(Outline, 'MiterLimit', function () { return 28.955; },
  function (v) { if (Math.abs(v - 28.955) > 0.01) __no('Outline.MiterLimit', 'the miter limit is fixed'); });

// Transparency: uniform from the object's opacity, the others from a
// transparency effect's mask.
__def(Transparency, 'Type', function () {
  const ids = this.o._ids();
  if (!ids.length) return cdrNoTransparency;
  const mask = __call('vg_info', { id: ids[0] }).transparency_mask;
  if (mask) {
    return { fountain: cdrFountainTransparency, pattern: cdrPatternTransparency,
      texture: cdrTextureTransparency }[mask] || cdrUniformTransparency;
  }
  return this.Uniform > 0 ? cdrUniformTransparency : cdrNoTransparency;
});
__def(Transparency, 'Fountain', function () {
  return { get Steps() { return 0; }, set Steps(v) { __no('Transparency.Fountain.Steps', 'fountains are drawn smooth'); } };
});

// Color: tints of spot colours.
__def(Color, 'Tint', function () { return this.v.model === 'spot' ? Math.round(this.v.tint * 100) : 100; },
  function (v) { if (this.v.model === 'spot') { const n = Object.assign({}, this.v, { tint: v / 100 }); this._set(n); } else __no('Color.Tint', 'only spot colours have a tint'); });

// Bitmaps.
const __modeType = { bw: cdrBlackAndWhiteImage, gray: cdrGrayscaleImage, paletted: cdrPalettedImage,
  rgb: cdrRGBColorImage, cmyk: cdrCMYKColorImage };
class __Bitmap {
  constructor(id) { this.id = id; }
  get _b() { return __call('vg_bitmap', { id: this.id }); }
  get SizeWidth() { return this._b.w; }
  get SizeHeight() { return this._b.h; }
  get ResolutionX() { const b = this._b; return Math.round(b.w / (b.width_mm / 25.4)); }
  get ResolutionY() { const b = this._b; return Math.round(b.h / (b.height_mm / 25.4)); }
  get Mode() { return __modeType[this._b.mode]; }
  get Transparent() { return this._b.transparent; }
  get ExternallyLinked() { return false; }
  get CropEnvelopeModified() { return false; }
  get Watermarked() { return false; }
  ResolveLink() { }
  Crop() { }
  ConvertTo(Mode) { __call('vg_bitmap_mode', { id: this.id, mode: __imageMode[Mode] || 'rgb' }); }
  // Resample(Width, Height, AntiAlias, ResolutionX, ResolutionY): pixel
  // sizes, or the resolutions at the bitmap's present size.
  Resample(Width, Height, AntiAlias, ResolutionX, ResolutionY) {
    const b = this._b;
    let w = Width || 0, h = Height || 0;
    if (!w && ResolutionX) w = b.width_mm / 25.4 * ResolutionX;
    if (!h && (ResolutionY || ResolutionX)) h = b.height_mm / 25.4 * (ResolutionY || ResolutionX);
    if (!w) w = b.w;
    if (!h) h = b.h;
    __call('vg_bitmap_resample', { id: this.id, w: w, h: h, anti_alias: AntiAlias === undefined ? true : !!AntiAlias });
  }
  ApplyBitmapEffect() { return __no('Bitmap.ApplyBitmapEffect', 'use the Effects menu'); }
}
__def(Shape, 'Bitmap', function () { return this._info().kind === 'bitmap' ? new __Bitmap(this.id) : null; });

// Curves: the nodes chosen with the Shape tool, areas.
class __NodeRange {
  constructor(id, indices) { this.id = id; this.indices = indices; }
  get Count() { return this.indices.length; }
  Item(Index) { const i = this.indices[Index - 1]; return i === undefined ? null : new __Node(this.id, i); }
  get FirstNode() { if (!this.indices.length) throw new Error('No node is selected'); return this.Item(1); }
  get LastNode() { if (!this.indices.length) throw new Error('No node is selected'); return this.Item(this.Count); }
  [Symbol.iterator]() { return __iter(this.indices.map(i => new __Node(this.id, i))); }
}
__ShapeCurve.prototype.Selection = function () { return new __NodeRange(this.id, __call('vg_node_selection', { id: this.id }).indices); };
__def(__ShapeCurve, 'Area', function () { return __call('vg_area', { id: this.id }).total / (__f() * __f()); });
__def(__SubPath, 'Area', function () { return __call('vg_area', { id: this.id }).subpaths[this.i] / (__f() * __f()); });
__def(__Nodes, 'FirstNode', function () { return this.First; });
__def(__Nodes, 'LastNode', function () { return this.Last; });

// Text: overflow, words, character ranges, line spacing.
__def(__Text, 'Overflow', function () { return __call('vg_overflow', { id: this.id }).overflow; });
class __TextRange {
  constructor(id, start, end) { this.id = id; this.start = start; this.end = end; }
  // Rebuild the spans so [start, end) is its own span(s), then let `f`
  // change those.
  _edit(f) {
    const k = __call('vg_kind', { id: this.id });
    const out = [];
    let pos = 0;
    for (const sp of k.spans) {
      const chars = Array.from(sp.text);
      const a = Math.max(0, Math.min(chars.length, this.start - pos));
      const b = Math.max(0, Math.min(chars.length, this.end - pos));
      const parts = [[0, a, false], [a, b, true], [b, chars.length, false]];
      for (const [x, y, inside] of parts) {
        if (y <= x) continue;
        const piece = Object.assign({}, sp, { text: chars.slice(x, y).join('') });
        if (inside) f(piece);
        out.push(piece);
      }
      pos += chars.length;
    }
    k.spans = out;
    __call('vg_set_kind', { id: this.id, kind: k });
  }
  _first() {
    const k = __call('vg_kind', { id: this.id });
    let pos = 0;
    for (const sp of k.spans) { const n = Array.from(sp.text).length; if (this.start < pos + n) return sp; pos += n; }
    return k.spans[k.spans.length - 1];
  }
  get Text() { return Array.from(new __Story(this.id).Text).slice(this.start, this.end).join(''); }
  get Start() { return this.start; }
  get End() { return this.end; }
  get Size() { return this._first().size_pt; }
  set Size(v) { this._edit(s => { s.size_pt = Number(v); }); }
  get Font() { return this._first().font_family; }
  set Font(v) { this._edit(s => { s.font_family = String(v); }); }
  get Bold() { return !!this._first().bold; }
  set Bold(v) { this._edit(s => { s.bold = !!v; }); }
  get Italic() { return !!this._first().italic; }
  set Italic(v) { this._edit(s => { s.italic = !!v; }); }
  get Fill() {
    const me = this;
    return { get UniformColor() { const f = me._first().fill; return new Color(f && f.fill === 'solid' ? __plain(f) : { model: 'rgb', r: 0, g: 0, b: 0 },
      (nv) => me._edit(s => { s.fill = Object.assign({ fill: 'solid' }, nv); })); } };
  }
}
__Story.prototype.Range = function (Start, End) { return new __TextRange(this.id, Start, End); };
__def(__Story, 'Words', function () {
  const t = Array.from(this.Text), list = [];
  let i = 0;
  while (i < t.length) {
    while (i < t.length && /\s/.test(t[i])) i++;
    const s = i;
    while (i < t.length && !/\s/.test(t[i])) i++;
    if (i > s) list.push(new __TextRange(this.id, s, i));
  }
  return { Count: list.length, Item: (n) => list[n - 1], [Symbol.iterator]: () => __iter(list) };
});
__def(__Story, 'Characters', function () {
  const n = Array.from(this.Text).length, id = this.id;
  const list = () => Array.from({ length: n }, (_, i) => new __TextRange(id, i, i + 1));
  return { Count: n, Item: (i) => new __TextRange(id, i - 1, i), [Symbol.iterator]: () => __iter(list()) };
});
__def(__Story, 'LineSpacing', function () { return this.k.para.leading_pct; },
  function (v) { const k = this.k; k.para.leading_pct = Number(v); this.put(k); });
__def(__Story, 'LanguageID', function () { return __no('Story.LanguageID', 'text has no language of its own'); });
Document.prototype.Close = function () { return __no('Document.Close', 'a macro works on the drawing that stays open'); };

// Moving between layers, stacking order, clip frames, picking.
Shape.prototype.MoveToLayer = function (Layer) { __call('vg_move_to_layer', { id: this.id, layer: Layer.id }); };
ShapeRange.prototype.MoveToLayer = function (Layer) { for (const id of this._ids()) __call('vg_move_to_layer', { id: id, layer: Layer.id }); };
Shape.prototype.OrderIsInFrontOf = function (Shape) {
  const a = __call('vg_zorder', { id: this.id }).rank, b = __call('vg_zorder', { id: __ids(Shape)[0] }).rank;
  return a > b;
};
Shape.prototype.AddToClipFrame = function (Shape) { __call('vg_place_inside', { ids: [this.id], frame: __ids(Shape)[0] }); };
ShapeRange.prototype.AddToClipFrame = function (Shape) { __call('vg_place_inside', { ids: this._ids(), frame: __ids(Shape)[0] }); };
for (const cls of [Shapes, ShapeRange]) {
  cls.prototype.FindAnyOfType = function (...Types) {
    const list = cls === Shapes ? this._list : this._ids();
    return new ShapeRange(list.filter(id => Types.includes(__typeOf(__call('vg_info', { id: id })))));
  };
}
Page.prototype.SelectShapesAtPoint = function (x, y, SelectUnfilled) {
  const px = __mm(x), py = __mm(y);
  const hit = __call('page_shapes', { page: this.id }).ids.filter(id => {
    const b = __call('vg_bounds', { ids: [id] });
    return px >= b.x0 && px <= b.x1 && py >= b.y0 && py <= b.y1;
  });
  __call('select', { ids: hit, add: false });
  return new ShapeRange(hit);
};
Layer.prototype.ImportEx = function (FileName) {
  const layer = this;
  return { Finish() { return layer.Import(FileName); }, Reset() { } };
};
__def(Document, 'DrawingOriginX', function () { return 0; }, function (v) { if (v) __no('Document.DrawingOriginX', 'the origin is the page corner'); });
__def(Document, 'DrawingOriginY', function () { return 0; }, function (v) { if (v) __no('Document.DrawingOriginY', 'the origin is the page corner'); });
__def(Document, 'DesktopLayer', function () { return __no('Document.DesktopLayer', 'there is no desktop layer'); });
__View.prototype.ToFitShapeRange = function () { };
__View.prototype.GetViewArea = function () { return __no('ActiveView.GetViewArea'); };
Application.Quit = function () { return __no('Application.Quit'); };
Color.prototype.UserAssignEx = function () { return __no('Color.UserAssignEx', 'macros cannot open the colour picker'); };
Color.prototype.UserAssign = Color.prototype.UserAssignEx;
Layer.prototype.PasteSpecial = function () { return __no('Layer.PasteSpecial', 'the clipboard is not open to macros'); };
__def(Fill, 'Pattern', function () { return { get Type() { return __no('Fill.Pattern'); } }; });
__def(Shape, 'StartShape', function () { return __no('Shape.StartShape', 'connectors are not objects of their own'); });
__Bitmap.prototype.GetPixelColor = function () { return __no('Bitmap.GetPixelColor'); };
