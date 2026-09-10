"""Measure a complete Rust exploration whose component callbacks are Python."""

import statistics
import time

import newtui


class PythonMixer:
    """Four five-position dials: 2,500 open states and 15,000 transitions."""

    def __init__(self):
        self.selected = 0
        self.levels = [0, 0, 0, 0]

    def handle(self, key):
        if key == newtui.Key.UP:
            self.selected = max(0, self.selected - 1)
        elif key == newtui.Key.DOWN:
            self.selected = min(len(self.levels) - 1, self.selected + 1)
        elif key == newtui.Key.LEFT:
            self.levels[self.selected] = max(
                0, self.levels[self.selected] - 1
            )
        elif key == newtui.Key.RIGHT:
            self.levels[self.selected] = min(
                4, self.levels[self.selected] + 1
            )
        elif key == newtui.Key.ENTER:
            return newtui.Flow.close(True)
        elif key == newtui.Key.ESC:
            return newtui.Flow.close(False)
        return newtui.Flow.stay()

    def view(self):
        return newtui.View(
            "mixer",
            [
                newtui.Row(
                    f"dial {index}",
                    str(level),
                    selected=index == self.selected,
                    adjustable=True,
                )
                for index, level in enumerate(self.levels)
            ],
            "arrows change",
        )

    def fingerprint(self):
        return f"{self.selected}:{','.join(map(str, self.levels))}"


def measure(repetitions=5):
    durations = []
    report = None
    for _ in range(repetitions):
        started = time.perf_counter()
        report = newtui.explore(
            lambda: PythonMixer(), newtui.properties.standard()
        )
        durations.append(time.perf_counter() - started)
        if not report.is_clean:
            raise RuntimeError(str(report))
    return report, statistics.median(durations), durations


if __name__ == "__main__":
    result, median, samples = measure()
    micros = median * 1_000_000 / result.transitions
    print(
        f"states={result.states} transitions={result.transitions} "
        f"median={median:.6f}s us_per_transition={micros:.3f} "
        f"samples={','.join(f'{sample:.6f}' for sample in samples)}"
    )
