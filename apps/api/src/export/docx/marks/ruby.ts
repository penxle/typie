import { XmlAttributeComponent, XmlComponent } from 'docx';
import { pxToHalfPt } from '../utils/unit';

class RubyAlignAttribute extends XmlAttributeComponent<{ readonly val: string }> {
  protected override readonly xmlKeys = { val: 'w:val' };
}

class RubyAlign extends XmlComponent {
  constructor(val = 'center') {
    super('w:rubyAlign');
    this.root.push(new RubyAlignAttribute({ val }));
  }
}

class HpsAttribute extends XmlAttributeComponent<{ readonly val: string }> {
  protected override readonly xmlKeys = { val: 'w:val' };
}

class Hps extends XmlComponent {
  constructor(val: string) {
    super('w:hps');
    this.root.push(new HpsAttribute({ val }));
  }
}

class HpsRaise extends XmlComponent {
  constructor(val: string) {
    super('w:hpsRaise');
    this.root.push(new HpsAttribute({ val }));
  }
}

class HpsBaseText extends XmlComponent {
  constructor(val: string) {
    super('w:hpsBaseText');
    this.root.push(new HpsAttribute({ val }));
  }
}

class Lid extends XmlComponent {
  constructor(val: string) {
    super('w:lid');
    this.root.push(new HpsAttribute({ val }));
  }
}

class RubyProperties extends XmlComponent {
  constructor() {
    super('w:rubyPr');
    this.root.push(new RubyAlign('center'), new Hps('10'), new HpsRaise('0'), new HpsBaseText('0'), new Lid('en-US'));
  }
}

class Bold extends XmlComponent {
  constructor() {
    super('w:b');
  }
}

class Italic extends XmlComponent {
  constructor() {
    super('w:i');
  }
}

class Strike extends XmlComponent {
  constructor() {
    super('w:strike');
  }
}

class UnderlineAttribute extends XmlAttributeComponent<{ readonly val: string }> {
  protected override readonly xmlKeys = { val: 'w:val' };
}

class Underline extends XmlComponent {
  constructor() {
    super('w:u');
    this.root.push(new UnderlineAttribute({ val: 'single' }));
  }
}

class FontSizeAttribute extends XmlAttributeComponent<{ readonly val: string }> {
  protected override readonly xmlKeys = { val: 'w:val' };
}

class FontSize extends XmlComponent {
  constructor(size: number) {
    super('w:sz');
    this.root.push(new FontSizeAttribute({ val: String(pxToHalfPt(size)) }));
  }
}

class FontFamilyAttribute extends XmlAttributeComponent<{ readonly ascii: string; readonly hAnsi: string }> {
  protected override readonly xmlKeys = { ascii: 'w:ascii', hAnsi: 'w:hAnsi' };
}

class FontFamily extends XmlComponent {
  constructor(fontName: string) {
    super('w:rFonts');
    this.root.push(new FontFamilyAttribute({ ascii: fontName, hAnsi: fontName }));
  }
}

class ColorAttribute extends XmlAttributeComponent<{ readonly val: string }> {
  protected override readonly xmlKeys = { val: 'w:val' };
}

class Color extends XmlComponent {
  constructor(color: string) {
    super('w:color');
    this.root.push(new ColorAttribute({ val: color.replace('#', '') }));
  }
}

class Text extends XmlComponent {
  constructor(text: string) {
    super('w:t');
    this.root.push(text);
  }
}

class RunProperties extends XmlComponent {
  constructor(options?: {
    bold?: boolean;
    italic?: boolean;
    underline?: boolean;
    strike?: boolean;
    fontSize?: number; // px
    fontFamily?: string;
    color?: string;
  }) {
    super('w:rPr');

    if (!options) return;

    if (options.bold) {
      this.root.push(new Bold());
    }

    if (options.italic) {
      this.root.push(new Italic());
    }

    if (options.underline) {
      this.root.push(new Underline());
    }

    if (options.strike) {
      this.root.push(new Strike());
    }

    if (options.fontSize) {
      this.root.push(new FontSize(options.fontSize));
    }

    if (options.fontFamily) {
      this.root.push(new FontFamily(options.fontFamily));
    }

    if (options.color) {
      this.root.push(new Color(options.color));
    }
  }
}

class RubyTextRun extends XmlComponent {
  constructor(text: string, fontSize?: number, fontFamily?: string, color?: string) {
    super('w:r');

    const runProps = new RunProperties({
      fontSize: fontSize ? Math.round(fontSize * 0.5) : 5,
      fontFamily,
      color,
    });

    this.root.push(runProps, new Text(text));
  }
}

class BaseTextRun extends XmlComponent {
  constructor(
    text: string,
    options?: {
      bold?: boolean;
      italic?: boolean;
      underline?: boolean;
      strike?: boolean;
      fontSize?: number; // px
      fontFamily?: string;
      color?: string;
    },
  ) {
    super('w:r');

    if (options) {
      const runProps = new RunProperties(options);
      this.root.push(runProps);
    }

    this.root.push(new Text(text));
  }
}

class RubyText extends XmlComponent {
  constructor(text: string, fontSize?: number, fontFamily?: string, color?: string) {
    super('w:rt');
    this.root.push(new RubyTextRun(text, fontSize, fontFamily, color));
  }
}

class RubyBase extends XmlComponent {
  constructor(
    text: string,
    options?: {
      bold?: boolean;
      italic?: boolean;
      underline?: boolean;
      strike?: boolean;
      fontSize?: number; // px
      fontFamily?: string;
      color?: string;
    },
  ) {
    super('w:rubyBase');
    this.root.push(new BaseTextRun(text, options));
  }
}

export class Ruby extends XmlComponent {
  constructor(
    baseText: string,
    rubyText: string,
    options?: {
      bold?: boolean;
      italic?: boolean;
      underline?: boolean;
      strike?: boolean;
      fontSize?: number; // px
      fontFamily?: string;
      color?: string;
    },
  ) {
    super('w:ruby');

    this.root.push(
      new RubyProperties(),
      new RubyText(rubyText, options?.fontSize, options?.fontFamily, options?.color),
      new RubyBase(baseText, options),
    );
  }
}

export class RubyRun extends XmlComponent {
  constructor(ruby: Ruby) {
    super('w:r');
    this.root.push(ruby);
  }
}
