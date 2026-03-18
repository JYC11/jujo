from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

# <ai:customize hint="Add imports for Order field types">
# </ai:customize>


@dataclass
class Order:
    """Represents a order entity."""

    id: str
    title: str = None
    amount: Optional[Decimal] = None
    created_at: datetime = None

    # <ai:customize hint="Add domain methods for Order">
    def validate(self) -> bool:
        raise NotImplementedError
    # </ai:customize>
