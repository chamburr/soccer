import sys

import numpy as np
from matplotlib import pyplot as plt
from scipy import linalg


def set_axes_equal(ax):
    limits = np.array(
        [
            ax.get_xlim3d(),
            ax.get_ylim3d(),
            ax.get_zlim3d(),
        ]
    )
    origin = np.mean(limits, axis=1)
    radius = 0.5 * np.max(np.abs(limits[:, 1] - limits[:, 0]))
    set_axes_radius(ax, origin, radius)


def set_axes_radius(ax, origin, radius):
    x, y, z = origin
    ax.set_xlim3d([x - radius, x + radius])
    ax.set_ylim3d([y - radius, y + radius])
    ax.set_zlim3d([z - radius, z + radius])


def ellipsoid_fit(s):
    D = np.array(
        [
            s[0] ** 2.0,
            s[1] ** 2.0,
            s[2] ** 2.0,
            2.0 * s[1] * s[2],
            2.0 * s[0] * s[2],
            2.0 * s[0] * s[1],
            2.0 * s[0],
            2.0 * s[1],
            2.0 * s[2],
            np.ones_like(s[0]),
        ]
    )

    S = np.dot(D, D.T)
    S_11 = S[:6, :6]
    S_12 = S[:6, 6:]
    S_21 = S[6:, :6]
    S_22 = S[6:, 6:]

    C = np.array(
        [
            [-1, 1, 1, 0, 0, 0],
            [1, -1, 1, 0, 0, 0],
            [1, 1, -1, 0, 0, 0],
            [0, 0, 0, -4, 0, 0],
            [0, 0, 0, 0, -4, 0],
            [0, 0, 0, 0, 0, -4],
        ]
    )

    E = np.dot(linalg.inv(C), S_11 - np.dot(S_12, np.dot(linalg.inv(S_22), S_21)))

    E_w, E_v = np.linalg.eig(E)

    v_1 = E_v[:, np.argmax(E_w)]
    if v_1[0] < 0:
        v_1 = -v_1

    v_2 = np.dot(np.dot(-np.linalg.inv(S_22), S_21), v_1)

    M = np.array(
        [
            [v_1[0], v_1[5], v_1[4]],
            [v_1[5], v_1[1], v_1[3]],
            [v_1[4], v_1[3], v_1[2]],
        ]
    )
    n = np.array([[v_2[0]], [v_2[1]], [v_2[2]]])
    d = v_2[3]

    return M, n, d


def main():
    F = 1000
    b = np.zeros([3, 1])
    A_1 = np.eye(3)

    if len(sys.argv) != 3:
        print("No file path or version provided")
        exit(0)

    path = sys.argv[1]
    version = int(sys.argv[2])
    data = np.loadtxt(path, delimiter=",")
    print("Shape of data array:", data.shape)
    print("First 5 rows raw:\n", data[:5])

    low = np.percentile(data.T, 1, axis=1) - 10
    high = np.percentile(data.T, 99, axis=1) + 10
    data = data[np.all((data >= low) & (data <= high), axis=1)]
    print("\nShape of filtered data array:", data.shape)
    print("Filtered data range:\n", np.array([low, high]).T)

    if version == 2:
        F = 1

    s = data.T
    M, n, d = ellipsoid_fit(s)

    M_1 = linalg.inv(M)
    b = -np.dot(M_1, n)
    A_1 = np.real(F / np.sqrt(np.dot(n.T, np.dot(M_1, n)) - d) * linalg.sqrtm(M))

    print("\nHard iron bias:\n", b)
    print("Soft iron transformation matrix:\n", A_1)

    plt.rcParams["figure.autolayout"] = True

    fig = plt.figure()
    ax = fig.add_subplot(111, projection="3d")
    ax.set_box_aspect([1, 1, 1])
    ax.scatter(data[:, 0], data[:, 1], data[:, 2], marker="o", color="r")
    set_axes_equal(ax)
    plt.show()

    result = []
    for row in data:
        xm_off = row[0] - b[0]
        ym_off = row[1] - b[1]
        zm_off = row[2] - b[2]

        xm_cal = xm_off * A_1[0, 0] + ym_off * A_1[0, 1] + zm_off * A_1[0, 2]
        ym_cal = xm_off * A_1[1, 0] + ym_off * A_1[1, 1] + zm_off * A_1[1, 2]
        zm_cal = xm_off * A_1[2, 0] + ym_off * A_1[2, 1] + zm_off * A_1[2, 2]

        result = np.append(result, np.array([xm_cal, ym_cal, zm_cal]))

    u = np.linspace(0, 2 * np.pi, 100)
    v = np.linspace(0, np.pi, 100)

    result = result.reshape(-1, 3)
    fig = plt.figure()
    ax = fig.add_subplot(111, projection="3d")
    ax.set_box_aspect([1, 1, 1])
    ax.scatter(result[:, 0], result[:, 1], result[:, 2], marker="o", color="g")
    ax.plot_surface(
        F * np.outer(np.cos(u), np.sin(v)),
        F * np.outer(np.sin(u), np.sin(v)),
        F * np.outer(np.ones(np.size(u)), np.cos(v)),
        color="b",
        alpha=0.25,
    )
    ax.set_xlim([-F * 1.2, F * 1.2])
    ax.set_ylim([-F * 1.2, F * 1.2])
    ax.set_zlim([-F * 1.2, F * 1.2])
    plt.show()

    print("\nData normalized to", F)
    print("Max values:", np.max(result, axis=0))
    print("Min values:", np.min(result, axis=0))

    print("\n*************************")
    print("Code to paste: ")
    print("*************************")

    b = np.round(b, 2)
    A_1 = np.round(A_1, 5)

    if version == 2:
        print(
            "\npub const ACC_OFFSET: [f32; 3] = "
            + f"[{b[0, 0]:.3f}, {b[1, 0]:.3f}, {b[2, 0]:.3f}];"
        )

        print(
            "pub const ACC_MISALIGNMENT: [[f32; 3]; 3] = [\n"
            + f"    [{A_1[0, 0]:.3f}, {A_1[0, 1]:.3f}, {A_1[0, 2]:.3f}],\n"
            + f"    [{A_1[1, 0]:.3f}, {A_1[1, 1]:.3f}, {A_1[1, 2]:.3f}],\n"
            + f"    [{A_1[2, 0]:.3f}, {A_1[2, 1]:.3f}, {A_1[2, 2]:.3f}],\n"
            + f"];\n"
        )
    else:
        print(
            "\npub const HARD_IRON_OFFSET: [f32; 3] = "
            + f"[{b[0, 0]:.3f}, {b[1, 0]:.3f}, {b[2, 0]:.3f}];"
        )

        print(
            "pub const SOFT_IRON_MATRIX: [[f32; 3]; 3] = [\n"
            + f"    [{A_1[0, 0]:.3f}, {A_1[0, 1]:.3f}, {A_1[0, 2]:.3f}],\n"
            + f"    [{A_1[1, 0]:.3f}, {A_1[1, 1]:.3f}, {A_1[1, 2]:.3f}],\n"
            + f"    [{A_1[2, 0]:.3f}, {A_1[2, 1]:.3f}, {A_1[2, 2]:.3f}],\n"
            + f"];\n"
        )


if __name__ == "__main__":
    main()
