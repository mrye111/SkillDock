import React from 'react';

/**
 * RibbonBackdrop
 * 复刻参考图中的「丝带元素」与流畅立体层次氛围背景：
 * - 柔和天蓝到珊瑚粉蜜桃色的氛围天幕底色；
 * - 具有体积光与双曲面扭曲感的 3D 渐变流动丝带（左下方蓝紫丝带环绕 + 右上方玫瑰蜜桃丝带延伸）；
 * - 细腻的微光散射层，为上层云母毛玻璃（Mica & Glassmorphism）提供丰富且纯净的透光质感。
 */
export const RibbonBackdrop: React.FC = () => {
  return (
    <div className="ribbon-backdrop-container pointer-events-none select-none" aria-hidden="true">
      {/* 1. 天幕环境光网格底色 */}
      <div className="ribbon-sky-gradient" />

      {/* 2. 空间柔光光晕 (Ambient Volumetric Glows) */}
      <div className="ribbon-glow-orb glow-top-left" />
      <div className="ribbon-glow-orb glow-top-right" />
      <div className="ribbon-glow-orb glow-bottom-left" />
      <div className="ribbon-glow-orb glow-center" />

      {/* 3. 矢量高精度 3D 丝带曲线层 (Flowing Volumetric Ribbons) */}
      <svg
        className="ribbon-svg-canvas"
        viewBox="0 0 1440 900"
        preserveAspectRatio="xMidYMid slice"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          {/* 丝带 1：左下方蓝紫流动环（Cool Indigo / Blue / Lavender Loop） */}
          <linearGradient id="coolRibbonBody" x1="0%" y1="100%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#4f46e5" stopOpacity="0.85" />
            <stop offset="35%" stopColor="#6366f1" stopOpacity="0.9" />
            <stop offset="70%" stopColor="#818cf8" stopOpacity="0.8" />
            <stop offset="100%" stopColor="#a78bfa" stopOpacity="0.75" />
          </linearGradient>

          <linearGradient id="coolRibbonHighlight" x1="20%" y1="0%" x2="80%" y2="100%">
            <stop offset="0%" stopColor="#93c5fd" stopOpacity="0.8" />
            <stop offset="50%" stopColor="#c4b5fd" stopOpacity="0.6" />
            <stop offset="100%" stopColor="#e0e7ff" stopOpacity="0.3" />
          </linearGradient>

          <linearGradient id="coolRibbonUnder" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#312e81" stopOpacity="0.7" />
            <stop offset="60%" stopColor="#4338ca" stopOpacity="0.65" />
            <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.5" />
          </linearGradient>

          {/* 丝带 2：右上方与右侧暖珊瑚蜜桃曲面（Warm Peach / Coral / Rose Twist） */}
          <linearGradient id="warmRibbonBody" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#fb7185" stopOpacity="0.85" />
            <stop offset="40%" stopColor="#f43f5e" stopOpacity="0.8" />
            <stop offset="75%" stopColor="#fb923c" stopOpacity="0.8" />
            <stop offset="100%" stopColor="#fdba74" stopOpacity="0.7" />
          </linearGradient>

          <linearGradient id="warmRibbonHighlight" x1="10%" y1="20%" x2="90%" y2="80%">
            <stop offset="0%" stopColor="#fecdd3" stopOpacity="0.85" />
            <stop offset="50%" stopColor="#fed7aa" stopOpacity="0.6" />
            <stop offset="100%" stopColor="#fff1f2" stopOpacity="0.4" />
          </linearGradient>

          <linearGradient id="warmRibbonFold" x1="100%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stopColor="#be123c" stopOpacity="0.65" />
            <stop offset="60%" stopColor="#e11d48" stopOpacity="0.5" />
            <stop offset="100%" stopColor="#f97316" stopOpacity="0.45" />
          </linearGradient>

          {/* 柔化高斯滤镜，营造丝滑空气感 */}
          <filter id="ribbonDepthBlur" x="-10%" y="-10%" width="120%" height="120%">
            <feGaussianBlur stdDeviation="6" result="blur" />
            <feComposite in="SourceGraphic" in2="blur" operator="over" />
          </filter>

          <filter id="ambientSoftBlur" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="38" />
          </filter>
        </defs>

        {/* 左侧深层底色环面 */}
        <path
          d="M-80 620 C 120 480, 240 680, 160 840 C 100 960, -40 920, -120 800 Z"
          fill="url(#coolRibbonUnder)"
          opacity="0.7"
        />

        {/* 左侧主曲面丝带：如丝般平滑卷曲 */}
        <path
          d="M-100 700 C 60 480, 280 500, 360 690 C 430 850, 260 980, 80 950 C -60 920, -140 820, -100 700 Z"
          fill="url(#coolRibbonBody)"
          filter="url(#ribbonDepthBlur)"
        />

        {/* 左侧迎光高光条纹 */}
        <path
          d="M-70 680 C 70 490, 260 520, 330 670 C 370 760, 260 840, 150 820 C 30 800, -50 750, -70 680 Z"
          fill="url(#coolRibbonHighlight)"
          opacity="0.65"
        />

        {/* 右上方背景深层渐变织带 */}
        <path
          d="M 1100 -60 C 1280 120, 1480 200, 1500 480 C 1510 650, 1340 780, 1180 720 C 1060 680, 1160 440, 1280 340 C 1380 260, 1260 100, 1100 -60 Z"
          fill="url(#warmRibbonFold)"
          opacity="0.6"
        />

        {/* 右侧前景主要立体扭转丝带 */}
        <path
          d="M 1040 -120 C 1190 60, 1420 180, 1400 450 C 1380 680, 1120 740, 940 820 C 820 880, 760 950, 720 1020 C 760 930, 880 840, 1060 760 C 1240 680, 1360 550, 1340 380 C 1320 200, 1120 100, 980 -50 Z"
          fill="url(#warmRibbonBody)"
          filter="url(#ribbonDepthBlur)"
        />

        {/* 右侧立体丝带高光迎光面 */}
        <path
          d="M 1070 -80 C 1200 80, 1380 200, 1370 420 C 1360 560, 1220 660, 1050 730 C 960 770, 920 810, 880 860 C 930 800, 1080 720, 1240 630 C 1330 540, 1350 390, 1320 240 C 1290 120, 1150 20, 1070 -80 Z"
          fill="url(#warmRibbonHighlight)"
          opacity="0.8"
        />

        {/* 穿透中心边缘的微弱流线，增加轻盈感 */}
        <path
          d="M 180 820 Q 520 740 920 880"
          stroke="url(#coolRibbonHighlight)"
          strokeWidth="3.5"
          fill="none"
          opacity="0.4"
          filter="url(#ribbonDepthBlur)"
        />
        <path
          d="M 240 880 Q 600 780 1020 930"
          stroke="url(#warmRibbonHighlight)"
          strokeWidth="2.5"
          fill="none"
          opacity="0.35"
        />
      </svg>

      {/* 4. 微粒子 / 细腻磨砂噪点层，避免色阶断层 */}
      <div className="ribbon-sheen-overlay" />
    </div>
  );
};
