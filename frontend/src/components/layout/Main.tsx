import * as React from 'react'
import { cn } from '@/lib/utils'

type MainProps = React.HTMLAttributes<HTMLElement> & {
  fixed?: boolean
  fluid?: boolean
}

export function Main({ fixed, className, fluid, ...props }: MainProps) {
  return (
    <main
      data-layout={fixed ? 'fixed' : 'auto'}
      className={cn(
        'px-4 py-4 sm:px-6 lg:px-8 flex-1 flex flex-col',
        fixed && 'flex grow flex-col overflow-hidden',
        !fluid && 'mx-auto w-full max-w-7xl',
        className
      )}
      {...props}
    />
  )
}
