def c_to_f(celsius: float) -> float:
    """Celsius to Fahrenheit, to two decimals."""
    return round(celsius * 9 / 5 + 32, 2)


def f_to_c(fahrenheit: float) -> float:
    """Fahrenheit to Celsius, to two decimals."""
    return round((fahrenheit - 32) * 5 / 9, 2)


def km_to_miles(km: float) -> float:
    """Kilometres to miles (1 km = 0.621371 mi), to two decimals."""
    return round(km * 0.621371, 2)
