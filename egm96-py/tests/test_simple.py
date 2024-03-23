import math

import numpy as np
from egm96 import load_embedded_egm96_grid15
from pytest import approx


def test_test():
    geoid = load_embedded_egm96_grid15()

    assert geoid.get_height(138.2839817085188, 37.12378643088312) == approx(
        39.27814,
        1e-3,
    )
    assert geoid.get_height(0.0, 0.0) == approx(
        17.16158,
        1e-3,
    )
    assert geoid.get_height(66.66, 55.55) == approx(
        -21.80307,
        1e-3,
    )

    assert math.isnan(geoid.get_height(0, 99.0))
    assert math.isnan(geoid.get_height(0, -99.0))

    assert geoid.get_heights(
        np.array([138.2839817085188, 0.0, 66.66]),
        np.array([37.12378643088312, 0.0, 55.55]),
    ) == approx(
        np.array(
            [
                39.27814,
                17.16158,
                -21.80307,
            ]
        ),
        1e-3,
    )


if __name__ == "__main__":
    test_test()
