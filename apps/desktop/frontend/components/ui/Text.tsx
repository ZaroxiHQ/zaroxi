import { cn } from '@/lib/utils';
import { ReactNode } from 'react';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';
import UI_TOKENS from '@/theme/tokens';

interface TextProps {
  children: ReactNode;
  className?: string;
  variant?: 'body' | 'label' | 'caption' | 'code' | 'heading';
  size?: 'xs' | 'sm' | 'base' | 'lg' | 'xl';
  weight?: 'normal' | 'medium' | 'semibold' | 'bold';
  family?: 'sans' | 'mono' | 'icon';
  align?: 'left' | 'center' | 'right';
}

export function Text({
  children,
  className,
  variant = 'body',
  size = 'base',
  weight = 'normal',
  family = 'sans',
  align = 'left',
}: TextProps) {
  const sizeClasses = {
    xs: 'text-xs',
    sm: 'text-sm',
    base: 'text-base',
    lg: 'text-lg',
    xl: 'text-xl',
  };

  const weightClasses = {
    normal: 'font-normal',
    medium: 'font-medium',
    semibold: 'font-semibold',
    bold: 'font-bold',
  };

  const familyStyles: Record<string, React.CSSProperties> = {
    sans: { fontFamily: FONT_TOKENS.ui },
    mono: { fontFamily: FONT_TOKENS.mono },
    icon: { fontFamily: FONT_TOKENS.icon },
  };

  const alignClasses = {
    left: 'text-left',
    center: 'text-center',
    right: 'text-right',
  };

  return (
    <div
      className={cn(
        sizeClasses[size],
        weightClasses[weight],
        alignClasses[align],
        className
      )}
      style={{ ...familyStyles[family], color: UI_TOKENS.textPrimary }}
    >
      {children}
    </div>
  );
}
