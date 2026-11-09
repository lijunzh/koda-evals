"""The one record type."""

from dataclasses import dataclass
from datetime import date
from decimal import Decimal


@dataclass(frozen=True)
class Transaction:
    day: date
    payee: str
    memo: str
    amount: Decimal
