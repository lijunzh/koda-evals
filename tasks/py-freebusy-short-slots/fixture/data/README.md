# Data exports

Copied nightly from the two systems busy time comes from.

- `calendar.ics`: Dana's calendar. `DTSTART` and `DTEND` are local times;
  `DTEND` is when the meeting ends.
- `bookings.csv`: the room-booking system's export for the whole floor, one
  row per booking, `date`, `start` and `end` in local time. The booking
  system works in whole minutes and records the last minute a room is
  booked, so `end` is that minute: a booking for 14:00 to 15:00 reads
  `14:00,14:59`.
