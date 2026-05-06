import { cn } from '@/lib/utils';
import { nerdFontIcons } from '@/lib/theme/nerd-font-icons';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';
import UI_TOKENS from '@/theme/tokens';

// Export the IconName type for use in other files
export type IconName = keyof typeof nerdFontIcons;

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
  label?: string;
  debug?: boolean;
}

export function Icon({ name, size = 16, className, label, debug = false }: IconProps) {
  const iconGlyph = nerdFontIcons[name] || '?';
  const defaultColor = debug ? '#ff6b6b' : UI_TOKENS.textSecondary;

  return (
    <span 
      className={cn(
        'inline-flex items-center justify-center antialiased',
        'leading-none tracking-normal',
        'select-none',
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
        color: defaultColor,
        fontFamily: FONT_TOKENS.icon,
        fontVariantLigatures: 'normal',
        fontFeatureSettings: '"liga" 1, "calt" 1',
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
