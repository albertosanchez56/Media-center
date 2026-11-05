import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { BridgeResult, BridgeService } from '../../bridge/bridge.service';

@Component({
  selector: 'app-bridge-diagnostics',
  templateUrl: './bridge-diagnostics.component.html',
  styleUrl: './bridge-diagnostics.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BridgeDiagnosticsComponent {
  private readonly bridge = inject(BridgeService);

  protected readonly checking = signal(false);
  protected readonly result = signal<BridgeResult | null>(null);

  protected async check(fail: boolean): Promise<void> {
    if (this.checking()) {
      return;
    }
    this.checking.set(true);
    this.result.set(null);
    try {
      this.result.set(await this.bridge.check(fail));
    } finally {
      this.checking.set(false);
    }
  }
}
