import { Component, inject, signal } from '@angular/core';
import { BridgeResult, BridgeService } from './bridge/bridge.service';

@Component({
  imports: [],
  selector: 'app-root',
  styleUrl: './app.scss',
  templateUrl: './app.html',
})
export class App {
  private readonly bridge = inject(BridgeService);

  protected readonly title = signal('Media Center');
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
