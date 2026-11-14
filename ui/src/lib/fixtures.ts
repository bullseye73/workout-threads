// 테스트용 기본값 (화면 코드에서는 쓰지 않는다).
import type { DayItemDto } from './bindings/DayItemDto'

export function dayItem(over: Partial<DayItemDto> = {}): DayItemDto {
  return { activityId: 'a', sport: 'cycling', km: 10, postStatus: 'none', title: '일요일 오전 라이딩', startTime: '09:00',
    movingS: 1800, speedKmh: 20, pace: null, coverCard: null, cardCount: 0, ...over }
}
