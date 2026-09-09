import React from 'react';

export const ICON_PATHS: Record<string, string> = {
  layers: '<path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1Z"/>',
  target: '<rect x="3" y="4" width="7" height="6" rx="1.4"/><rect x="14" y="14" width="7" height="6" rx="1.4"/><path d="M6.5 10v7h7.5M14 7h3.5v7"/>',
  history: '<path d="M3.5 10a8.5 8.5 0 1 1 1 7M3 4v6h6M12 7v5l3.5 2"/>',
  settings: '<path d="m9 3-.5 2-2 1-2-.5-2 3 1.5 1.5v3L2.5 15l2 3 2-.5 2 1 .5 2h4l.5-2 2-1 2 .5 2-3-1.5-2v-3L20 8.5l-2-3-2 .5-2-1-.5-2Z"/><circle cx="11.3" cy="12" r="3"/>',
  search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4.5 4.5"/>',
  refresh: '<path d="M20 5v5h-5M4 19v-5h5"/><path d="M5.5 8a7 7 0 0 1 11.7-3L20 10M4 14l2.8 5A7 7 0 0 0 18.5 16"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  arrow: '<path d="M4 12h15m-5-5 5 5-5 5"/>',
  chevron: '<path d="m9 5 7 7-7 7"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  chevrons: '<path d="m8 8 4-4 4 4M8 16l4 4 4-4"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  circleCheck: '<circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  warning: '<path d="m12 3 10 18H2L12 3Z"/><path d="M12 9v5m0 3v.1"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10v.1"/>',
  code: '<path d="m8 6-6 6 6 6M16 6l6 6-6 6M14 4l-4 16"/>',
  document: '<path d="M6 3h8l4 4v14H6Z"/><path d="M14 3v5h4M9 12h6m-6 4h6"/>',
  rocket: '<path d="M9 15c-1-4 3-11 11-11 0 8-7 12-11 11Zm0 0-2-5-4 5 6 0Zm0 0 5 2-5 4v-6"/><circle cx="15" cy="9" r="1.5"/>',
  pen: '<path d="m4 20 4-1L20 7l-4-4L4 15v5Zm9-14 4 4M4 15l4 4"/>',
  brackets: '<path d="M8 4H4v16h4m8-16h4v16h-4M9 12h6"/>',
  flask: '<path d="M9 3h6M10 3v6L4 19q-1 2 2 2h12q3 0 2-2L14 9V3M8 14h8"/>',
  grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
  shield: '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z"/><path d="m8 12 3 3 5-6"/>',
  link: '<path d="m10 8 3-3a4 4 0 0 1 6 6l-3 3m-2 2-3 3a4 4 0 0 1-6-6l3-3m0 6 8-8"/>',
  external: '<path d="M14 3h7v7m0-7L10 14M9 4H4v16h16v-5"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M15 8V3H3v12h5"/>',
  more: '<circle cx="5" cy="12" r=".9"/><circle cx="12" cy="12" r=".9"/><circle cx="19" cy="12" r=".9"/>',
  pin: '<path d="m9 3 9 9-3 1-3 5-2-4-4-2 5-3 1-3M10 14l-6 6"/>',
  trash: '<path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7"/>',
  keyboard: '<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.1m4 0h.1m4 0h.1m4 0h.1M6 12h.1m4 0h.1m4 0h.1m4 0h.1M7 16h10"/>',
  upload: '<path d="M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6"/>',
  minus: '<path d="M5 12h14"/>',
  codex: '<path d="m12 2 9 5v10l-9 5-9-5V7l9-5Zm0 4 5 3v6l-5 3-5-3V9l5-3Z"/><path d="m7 9 5 3 5-3m-5 3v6"/>',
  claude_code: '<path d="M12 2v20M2 12h20M5 5l14 14M5 19 19 5M8 3l8 18M3 8l18 8M3 16l18-8M8 21l8-18"/>',
  cursor: '<path d="m4 3 16 10-7 1-3 7-6-18Z"/><path d="m4 3 9 11m0 0 5 7"/>',
  copilot_vscode: '<path d="M7 7V5c0-3 10-3 10 0v2M5 7h14v10H5Z"/><rect x="2" y="9" width="3" height="6" rx="1"/><rect x="19" y="9" width="3" height="6" rx="1"/><circle cx="9" cy="11" r="1"/><circle cx="15" cy="11" r="1"/><path d="M8 17v3h8v-3"/>',
  custom: '<path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3Z"/><path d="M8 13h8m-4-4v8"/>',
};

export interface IconProps {
  name: string;
  className?: string;
  size?: number;
  style?: React.CSSProperties;
}

export const Icon: React.FC<IconProps> = ({ name, className = '', size = 19, style }) => {
  const pathData = ICON_PATHS[name] || ICON_PATHS.document;

  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      style={{ width: size, height: size, ...style }}
      dangerouslySetInnerHTML={{ __html: pathData }}
    />
  );
};

export const BrandLogo: React.FC<{ className?: string; size?: number }> = ({
  className = '',
  size = 30,
}) => {
  return (
    <svg
      className={className}
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
      style={{ width: size, height: size }}
    >
      <path
        d="m16 3 12 7-12 7L4 10 16 3Z"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinejoin="round"
      />
      <path
        d="m4 16 12 7 12-7M4 22l12 7 12-7"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="26" cy="5" r="3" fill="#5A7BF0" stroke="#F9FBFF" strokeWidth="2" />
    </svg>
  );
};

export const ToolIcon: React.FC<{
  adapterId: string;
  large?: boolean;
  className?: string;
}> = ({ adapterId, large = false, className = '' }) => {
  return (
    <span
      className={`tool-icon ${adapterId} ${large ? 'large' : ''} ${className}`}
    >
      <Icon name={adapterId in ICON_PATHS ? adapterId : 'custom'} size={large ? 24 : 19} />
    </span>
  );
};
