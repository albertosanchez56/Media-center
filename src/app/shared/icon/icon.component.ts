import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

const PATHS = {
  logo: 'M4 5h16v14H4z M10 9v6l5-3z',
  music: 'M9 18V5l11-2v13 M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0z M20 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
  video: 'M3 6h13v12H3z M16 10l5-3v10l-5-3',
  movie: 'M3 4h18v16H3z M7 4v16 M17 4v16 M3 9h4 M3 15h4 M17 9h4 M17 15h4',
  playlist: 'M3 6h12 M3 12h12 M3 18h8 M17 14v6 M17 14l4-1',
  diagnostics: 'M3 12h4l3-7 4 14 3-7h4',
  play: 'M7 4l13 8-13 8z',
  previous: 'M18 5v14L8 12z M6 5v14',
  next: 'M6 5v14l10-7z M18 5v14',
  volume: 'M4 9h4l5-4v14l-5-4H4z M17 9a4 4 0 0 1 0 6',
  'now-playing': 'M4 4h16v16H4z M8 15l3-4 2 3 2-2 3 3',
} as const;

export type IconName = keyof typeof PATHS;

/** Icono decorativo dibujado con trazados propios. El nombre accesible lo pone el control que lo contiene. */
@Component({
  selector: 'app-icon',
  template: `
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path [attr.d]="path()" />
    </svg>
  `,
  styles: `
    :host {
      display: inline-flex;
      flex-shrink: 0;
      width: var(--icon-size);
      height: var(--icon-size);
    }

    svg {
      width: 100%;
      height: 100%;
      fill: none;
      stroke: currentColor;
      stroke-width: 1.8;
      stroke-linecap: round;
      stroke-linejoin: round;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IconComponent {
  readonly name = input.required<IconName>();

  protected readonly path = computed(() => PATHS[this.name()]);
}
