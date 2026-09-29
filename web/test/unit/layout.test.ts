import { describe, expect, it } from 'vitest'
import { parseContentAlign } from '~/utils/layout'

describe('parseContentAlign', () => {
  it('保存された値を読み、知らない値は中央にする', () => {
    expect(parseContentAlign('left')).toBe('left')
    expect(parseContentAlign('center')).toBe('center')
    expect(parseContentAlign(null)).toBe('center')
    expect(parseContentAlign('right')).toBe('center')
  })
})
