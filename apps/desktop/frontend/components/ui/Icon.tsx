import { cn } from '@/lib/utils';
import { nerdFontIcons } from '@/lib/theme/nerd-font-icons';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';

// Export the IconName type for use in other files
export type IconName = keyof typeof nerdFontIcons;

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
  label?: string;
  debug?: boolean;
}

/**
 * Icon
 *
 * Small, predictable icon primitive backed by the project's Nerd‑Font mapping.
 * Important:
 * - Prefer using color via CSS classes (e.g., "text-accent", "text-primary") so
 *   the theme tokens remain authoritative.
 * - Only apply an inline fallback color when no text color class is provided.
 */
export function Icon({ name, size = 16, className, label, debug = false }: IconProps) {
  const iconGlyph = nerdFontIcons[name] || '?';
  const defaultColor = debug ? '#ff6b6b' : 'var(--color-text-muted)';

  // If a consumer supplied a Tailwind/text-* class, let that control the color.
  // Detect basic "text-" tokens in the provided className.
  const hasTextColorClass = typeof className === 'string' && /\btext-/.test(className);

  const computedColor = hasTextColorClass ? undefined : defaultColor;

  return (
    <span 
      className={cn(
        'inline-flex items-center justify-center antialiased',
        'leading-none tracking-normal',
        'select-none transition-colors',
        debug && 'outline outline-1 outline-red-500',
        className
      )}
      style={{ 
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontSize: size,
        width: size,
        height: size,
        // Use inline color only as a fallback when no CSS text color is applied.
        color: computedColor as any,
        fontFamily: FONT_TOKENS.icon,
        fontVariantLigatures: 'normal',
        fontFeatureSettings: '"liga" 1, "calt" 1',
        lineHeight: 1,
      }}
      role="img"
      aria-label={label || name}
      title={label || name}
      data-icon-name={name}
    >
      {iconGlyph}
    </span>
  );
}
