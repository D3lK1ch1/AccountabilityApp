function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  }
  return `${secs}s`;
}

describe('formatDuration', () => {
  test('formats seconds only', () => {
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(5)).toBe('5s');
    expect(formatDuration(0)).toBe('0s');
  });

  test('formats minutes and seconds', () => {
    expect(formatDuration(60)).toBe('1m 0s');
    expect(formatDuration(125)).toBe('2m 5s');
    expect(formatDuration(3665)).toBe('1h 1m');
  });

  test('formats hours and minutes', () => {
    expect(formatDuration(3600)).toBe('1h 0m');
    expect(formatDuration(3665)).toBe('1h 1m');
    expect(formatDuration(7200)).toBe('2h 0m');
    expect(formatDuration(7325)).toBe('2h 2m');
  });

  test('handles edge cases', () => {
    expect(formatDuration(59)).toBe('59s');
    expect(formatDuration(3599)).toBe('59m 59s');
    expect(formatDuration(86399)).toBe('23h 59m');
  });
});
