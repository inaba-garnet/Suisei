import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Button } from './Button.vue'

// 見た目は Design の Button に合わせる（docs/web.md）
export const buttonVariants = cva(
  'inline-flex shrink-0 cursor-pointer items-center justify-center gap-1.5 rounded-md border font-medium whitespace-nowrap text-fg transition-all outline-none focus-visible:shadow-focus disabled:pointer-events-none disabled:opacity-40 [&_svg]:pointer-events-none [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        default:
          'border-border-strong bg-accent-soft hover:border-accent-base hover:bg-surface-4 hover:shadow-glow-soft active:border-accent-active',
        secondary: 'border-border-default bg-surface-2 hover:border-border-strong hover:bg-surface-3',
        ghost: 'border-transparent bg-transparent text-fg-muted hover:bg-surface-2 hover:text-fg',
        destructive: 'border-danger/35 bg-danger-bg text-danger hover:border-danger/60 hover:bg-danger/15',
      },
      size: {
        'default': 'h-8 px-3 text-body',
        'sm': 'h-6 px-2.5 text-body-sm',
        'lg': 'h-10 px-4 text-body',
        'xl': 'h-12 px-5 text-body-lg',
        'icon': 'size-8',
        'icon-lg': 'size-10',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
)

export type ButtonVariants = VariantProps<typeof buttonVariants>
