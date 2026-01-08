/**
 * ANSI escape sequence to HTML conversion utilities.
 *
 * Converts ANSI escape sequences (colors, bold, etc.) to HTML spans with inline styles.
 * Also handles the ^[ notation used in BBS systems.
 */

// ANSI color codes to CSS colors
const ANSI_COLORS: Record<number, string> = {
  30: '#000000', // Black
  31: '#cc0000', // Red
  32: '#00cc00', // Green
  33: '#cccc00', // Yellow
  34: '#0000cc', // Blue
  35: '#cc00cc', // Magenta
  36: '#00cccc', // Cyan
  37: '#cccccc', // White
  // Bright colors
  90: '#666666', // Bright Black
  91: '#ff0000', // Bright Red
  92: '#00ff00', // Bright Green
  93: '#ffff00', // Bright Yellow
  94: '#0000ff', // Bright Blue
  95: '#ff00ff', // Bright Magenta
  96: '#00ffff', // Bright Cyan
  97: '#ffffff', // Bright White
};

const ANSI_BG_COLORS: Record<number, string> = {
  40: '#000000', // Black
  41: '#cc0000', // Red
  42: '#00cc00', // Green
  43: '#cccc00', // Yellow
  44: '#0000cc', // Blue
  45: '#cc00cc', // Magenta
  46: '#00cccc', // Cyan
  47: '#cccccc', // White
  // Bright backgrounds
  100: '#666666',
  101: '#ff0000',
  102: '#00ff00',
  103: '#ffff00',
  104: '#0000ff',
  105: '#ff00ff',
  106: '#00ffff',
  107: '#ffffff',
};

interface AnsiState {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  blink: boolean;
  reverse: boolean;
  fgColor: string | null;
  bgColor: string | null;
}

/**
 * Convert ^[ notation to actual ESC character (0x1B).
 */
export function convertCaretEscape(text: string): string {
  return text.replace(/\^\[/g, '\x1b');
}

/**
 * Generate CSS style string from ANSI state.
 */
function stateToStyle(state: AnsiState): string {
  const styles: string[] = [];

  if (state.bold) {
    styles.push('font-weight: bold');
  }
  if (state.italic) {
    styles.push('font-style: italic');
  }
  if (state.underline) {
    styles.push('text-decoration: underline');
  }
  if (state.fgColor) {
    styles.push(`color: ${state.fgColor}`);
  }
  if (state.bgColor) {
    styles.push(`background-color: ${state.bgColor}`);
  }

  return styles.join('; ');
}

/**
 * Check if state has any styling.
 */
function hasStyle(state: AnsiState): boolean {
  return (
    state.bold ||
    state.italic ||
    state.underline ||
    state.blink ||
    state.reverse ||
    state.fgColor !== null ||
    state.bgColor !== null
  );
}

/**
 * Parse ANSI SGR (Select Graphic Rendition) codes and update state.
 */
function parseSgrCodes(codes: number[], state: AnsiState): void {
  for (let i = 0; i < codes.length; i++) {
    const code = codes[i];

    if (code === 0) {
      // Reset
      state.bold = false;
      state.italic = false;
      state.underline = false;
      state.blink = false;
      state.reverse = false;
      state.fgColor = null;
      state.bgColor = null;
    } else if (code === 1) {
      state.bold = true;
    } else if (code === 3) {
      state.italic = true;
    } else if (code === 4) {
      state.underline = true;
    } else if (code === 5 || code === 6) {
      state.blink = true;
    } else if (code === 7) {
      state.reverse = true;
    } else if (code === 22) {
      state.bold = false;
    } else if (code === 23) {
      state.italic = false;
    } else if (code === 24) {
      state.underline = false;
    } else if (code === 25) {
      state.blink = false;
    } else if (code === 27) {
      state.reverse = false;
    } else if (code >= 30 && code <= 37) {
      // Standard foreground colors
      state.fgColor = ANSI_COLORS[code] || null;
    } else if (code === 38) {
      // Extended foreground color
      if (codes[i + 1] === 5 && codes[i + 2] !== undefined) {
        // 256-color mode: 38;5;n
        const colorIndex = codes[i + 2];
        state.fgColor = get256Color(colorIndex);
        i += 2;
      } else if (codes[i + 1] === 2 && codes.length >= i + 5) {
        // RGB mode: 38;2;r;g;b
        const r = codes[i + 2];
        const g = codes[i + 3];
        const b = codes[i + 4];
        state.fgColor = `rgb(${r}, ${g}, ${b})`;
        i += 4;
      }
    } else if (code === 39) {
      // Default foreground color
      state.fgColor = null;
    } else if (code >= 40 && code <= 47) {
      // Standard background colors
      state.bgColor = ANSI_BG_COLORS[code] || null;
    } else if (code === 48) {
      // Extended background color
      if (codes[i + 1] === 5 && codes[i + 2] !== undefined) {
        // 256-color mode: 48;5;n
        const colorIndex = codes[i + 2];
        state.bgColor = get256Color(colorIndex);
        i += 2;
      } else if (codes[i + 1] === 2 && codes.length >= i + 5) {
        // RGB mode: 48;2;r;g;b
        const r = codes[i + 2];
        const g = codes[i + 3];
        const b = codes[i + 4];
        state.bgColor = `rgb(${r}, ${g}, ${b})`;
        i += 4;
      }
    } else if (code === 49) {
      // Default background color
      state.bgColor = null;
    } else if (code >= 90 && code <= 97) {
      // Bright foreground colors
      state.fgColor = ANSI_COLORS[code] || null;
    } else if (code >= 100 && code <= 107) {
      // Bright background colors
      state.bgColor = ANSI_BG_COLORS[code] || null;
    }
  }
}

/**
 * Get color from 256-color palette.
 */
function get256Color(index: number): string {
  if (index < 16) {
    // Standard colors
    const colors = [
      '#000000', '#cc0000', '#00cc00', '#cccc00',
      '#0000cc', '#cc00cc', '#00cccc', '#cccccc',
      '#666666', '#ff0000', '#00ff00', '#ffff00',
      '#0000ff', '#ff00ff', '#00ffff', '#ffffff',
    ];
    return colors[index] || '#ffffff';
  } else if (index < 232) {
    // 216 color cube (6x6x6)
    const i = index - 16;
    const r = Math.floor(i / 36) * 51;
    const g = Math.floor((i % 36) / 6) * 51;
    const b = (i % 6) * 51;
    return `rgb(${r}, ${g}, ${b})`;
  } else {
    // Grayscale (24 shades)
    const gray = (index - 232) * 10 + 8;
    return `rgb(${gray}, ${gray}, ${gray})`;
  }
}

/**
 * Escape HTML special characters.
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Convert ANSI escape sequences to HTML.
 *
 * @param text - Text containing ANSI escape sequences
 * @returns HTML string with spans for styling
 */
export function ansiToHtml(text: string): string {
  // First convert ^[ to ESC
  const converted = convertCaretEscape(text);

  const state: AnsiState = {
    bold: false,
    italic: false,
    underline: false,
    blink: false,
    reverse: false,
    fgColor: null,
    bgColor: null,
  };

  let result = '';
  let spanOpen = false;
  let i = 0;

  while (i < converted.length) {
    // Check for ESC sequence
    if (converted[i] === '\x1b' && converted[i + 1] === '[') {
      // Find the end of the sequence (letter)
      let j = i + 2;
      while (j < converted.length && !/[A-Za-z]/.test(converted[j])) {
        j++;
      }

      if (j < converted.length) {
        const command = converted[j];
        const params = converted.slice(i + 2, j);

        if (command === 'm') {
          // SGR (Select Graphic Rendition)
          // Close previous span if open
          if (spanOpen) {
            result += '</span>';
            spanOpen = false;
          }

          // Parse codes
          const codes = params
            .split(';')
            .map((s) => parseInt(s, 10) || 0);
          parseSgrCodes(codes, state);

          // Open new span if needed
          if (hasStyle(state)) {
            const style = stateToStyle(state);
            result += `<span style="${style}">`;
            spanOpen = true;
          }
        }
        // Skip other escape sequences (cursor movement, etc.)

        i = j + 1;
        continue;
      }
    }

    // Regular character
    result += escapeHtml(converted[i]);
    i++;
  }

  // Close any open span
  if (spanOpen) {
    result += '</span>';
  }

  return result;
}

/**
 * Check if text contains ANSI escape sequences or ^[ notation.
 */
export function hasAnsiCodes(text: string): boolean {
  return /\x1b\[|\\^\[/.test(text);
}
