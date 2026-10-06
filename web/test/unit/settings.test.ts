import { describe, expect, it } from 'vitest'
import { intervalLabel, withCurrent } from '~/utils/settings'

describe('intervalLabel', () => {
  it('割り切れる大きい単位で出す', () => {
    expect(intervalLabel(0)).toBe('止める')
    expect(intervalLabel(900)).toBe('15 分')
    expect(intervalLabel(3600)).toBe('1 時間')
    expect(intervalLabel(5400)).toBe('90 分')
    expect(intervalLabel(86400 * 3)).toBe('3 日')
    expect(intervalLabel(61)).toBe('61 秒')
  })
})

describe('withCurrent', () => {
  it('候補にない今の値を順に足し、止めるは末尾に残す', () => {
    expect(withCurrent([60, 3600, 0], 3600)).toEqual([60, 3600, 0])
    expect(withCurrent([60, 3600, 0], 120)).toEqual([60, 120, 3600, 0])
    expect(withCurrent([1, 7], 3)).toEqual([1, 3, 7])
  })
})
