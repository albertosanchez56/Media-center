import { ChangeDetectionStrategy, Component } from '@angular/core';
import { IconComponent } from '../../shared/icon/icon.component';

@Component({
  selector: 'app-player-bar',
  imports: [IconComponent],
  templateUrl: './player-bar.component.html',
  styleUrl: './player-bar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PlayerBarComponent {
  protected readonly unavailable = 'El reproductor estará disponible más adelante';
}
