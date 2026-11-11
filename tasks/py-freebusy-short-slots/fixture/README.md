# freebusy

Free and busy time for a person's working day (09:00-17:00), from their
calendar export and the floor's room bookings. See `data/README.md` for the
export formats.

    python3 -m freebusy data dana 2026-11-12
    python3 -m freebusy data dana 2026-11-12 --min 15

Tests: `python3 -m unittest -v`
