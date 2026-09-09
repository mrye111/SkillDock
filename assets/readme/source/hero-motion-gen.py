# 生成 hero GIF 帧：以 hero.svg 为底稿，按时间轴修改三个动画层的属性。
# 用法：python assets/readme/source/hero-motion-gen.py（输出到 assets/readme/frames/）
import io, math, os, re, subprocess, sys

ROOT = r"D:\code\SkillDock\assets\readme"
SRC = os.path.join(ROOT, "hero.svg")
OUT = os.path.join(ROOT, "frames")
FPS = 15
DURATION = 5.0
N = int(FPS * DURATION)  # 75 帧

def ease_out(t: float) -> float:
    return 1 - (1 - t) ** 3

def clamp(x, lo=0.0, hi=1.0):
    return max(lo, min(hi, x))

def frame_state(t: float):
    """返回 (chip_dy, chip_op, check_op, row2_op, exit_fade)"""
    # 0.15–1.05s 芯片下落（-70 → 0，ease-out），前 0.3s 淡入
    chip_t = ease_out(clamp((t - 0.15) / 0.9))
    chip_dy = -70 * (1 - chip_t)
    chip_op = clamp((t - 0.15) / 0.3)
    # 1.05–1.45s 对勾淡入
    check_op = clamp((t - 1.05) / 0.4)
    # 1.45–2.2s 其余三坞淡入
    row2_op = clamp((t - 1.45) / 0.75)
    # 4.3–4.9s 整体淡出
    exit_fade = 1 - clamp((t - 4.3) / 0.6)
    return chip_dy, chip_op, check_op, row2_op, exit_fade

def main():
    base = io.open(SRC, encoding="utf-8").read()
    os.makedirs(OUT, exist_ok=True)
    for i in range(N):
        t = i / FPS
        dy, chip_op, check_op, row2_op, fade = frame_state(t)
        s = base
        # 芯片层：位移 + 透明度
        s = s.replace('<g id="anim-chip">',
                      f'<g id="anim-chip" transform="translate(0 {dy:.1f})" opacity="{chip_op * fade:.3f}">')
        # 对勾层：透明度
        s = s.replace('<g id="anim-check" opacity="1">',
                      f'<g id="anim-check" opacity="{check_op * fade:.3f}">')
        # 三坞行：透明度
        s = s.replace('<g id="dock-row-2" opacity="1">',
                      f'<g id="dock-row-2" opacity="{row2_op * fade:.3f}">')
        io.open(os.path.join(OUT, f"f{i:03d}.svg"), "w", encoding="utf-8", newline="\n").write(s)
    print(f"{N} frame SVGs written to {OUT}")

if __name__ == "__main__":
    main()
