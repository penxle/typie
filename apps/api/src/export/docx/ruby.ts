/* eslint-disable unicorn/prefer-single-call -- ImportedXmlComponent.push() accepts only one argument */
import { ImportedXmlComponent } from 'docx';

const DEFAULT_BASE_SIZE_HP = 21; // 10.5pt in half-points (default CJK body text)

function buildRunProps(opts: Record<string, unknown>): ImportedXmlComponent {
  const rPr = new ImportedXmlComponent('w:rPr');

  if (opts.bold) {
    rPr.push(new ImportedXmlComponent('w:b'));
  }
  if (opts.italics) {
    rPr.push(new ImportedXmlComponent('w:i'));
  }
  if (opts.strike) {
    rPr.push(new ImportedXmlComponent('w:strike'));
  }
  if (opts.underline) {
    rPr.push(new ImportedXmlComponent('w:u', { 'w:val': 'single' }));
  }
  if (typeof opts.size === 'number') {
    rPr.push(new ImportedXmlComponent('w:sz', { 'w:val': String(opts.size) }));
  }
  if (typeof opts.font === 'string') {
    rPr.push(new ImportedXmlComponent('w:rFonts', { 'w:ascii': opts.font, 'w:eastAsia': opts.font, 'w:hAnsi': opts.font }));
  }
  if (typeof opts.color === 'string') {
    rPr.push(new ImportedXmlComponent('w:color', { 'w:val': opts.color }));
  }

  return rPr;
}

/**
 * w:ruby XML 구조를 생성한다.
 *
 * <w:r>
 *   <w:ruby>
 *     <w:rubyPr>
 *       <w:rubyAlign w:val="center"/>
 *       <w:hps w:val="{rubySize}"/>
 *       <w:hpsRaise w:val="{raise}"/>
 *       <w:hpsBaseText w:val="{baseSize}"/>
 *     </w:rubyPr>
 *     <w:rt><w:r><w:rPr>...</w:rPr><w:t>{rubyText}</w:t></w:r></w:rt>
 *     <w:rubyBase><w:r><w:rPr>...</w:rPr><w:t>{baseText}</w:t></w:r></w:rubyBase>
 *   </w:ruby>
 * </w:r>
 */
export function createRubyRun(baseText: string, rubyText: string, baseRunOptions: Record<string, unknown> = {}): ImportedXmlComponent {
  const baseSizeHp = typeof baseRunOptions.size === 'number' ? (baseRunOptions.size as number) : DEFAULT_BASE_SIZE_HP;
  const rubySizeHp = Math.round(baseSizeHp / 2);
  const raiseHp = baseSizeHp;

  // w:rubyPr
  const rubyPr = new ImportedXmlComponent('w:rubyPr');
  rubyPr.push(new ImportedXmlComponent('w:rubyAlign', { 'w:val': 'center' }));
  rubyPr.push(new ImportedXmlComponent('w:hps', { 'w:val': String(rubySizeHp) }));
  rubyPr.push(new ImportedXmlComponent('w:hpsRaise', { 'w:val': String(raiseHp) }));
  rubyPr.push(new ImportedXmlComponent('w:hpsBaseText', { 'w:val': String(baseSizeHp) }));

  // ruby rPr (smaller font size)
  const rubyRunOpts = { ...baseRunOptions, size: rubySizeHp };
  const rubyRPr = buildRunProps(rubyRunOpts);

  // w:rt (ruby text)
  const rtRun = new ImportedXmlComponent('w:r');
  rtRun.push(rubyRPr);
  const rtText = new ImportedXmlComponent('w:t', { 'xml:space': 'preserve' });
  rtText.push(rubyText);
  rtRun.push(rtText);

  const rt = new ImportedXmlComponent('w:rt');
  rt.push(rtRun);

  // w:rubyBase (base text)
  const baseRPr = buildRunProps(baseRunOptions);
  const baseRun = new ImportedXmlComponent('w:r');
  baseRun.push(baseRPr);
  const baseT = new ImportedXmlComponent('w:t', { 'xml:space': 'preserve' });
  baseT.push(baseText);
  baseRun.push(baseT);

  const rubyBase = new ImportedXmlComponent('w:rubyBase');
  rubyBase.push(baseRun);

  // w:ruby
  const ruby = new ImportedXmlComponent('w:ruby');
  ruby.push(rubyPr);
  ruby.push(rt);
  ruby.push(rubyBase);

  // outer w:r
  const outerRun = new ImportedXmlComponent('w:r');
  outerRun.push(ruby);

  return outerRun;
}
