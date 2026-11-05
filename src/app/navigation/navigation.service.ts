import { Injectable, computed, signal } from '@angular/core';
import { LIBRARIES, Library, Section } from './navigation';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly current = signal<Section>('music');

  readonly section = this.current.asReadonly();

  /** Biblioteca visible, o `null` si la zona central muestra otra sección. */
  readonly currentLibrary = computed<Library | null>(
    () => LIBRARIES.find((library) => library.id === this.current()) ?? null,
  );

  select(section: Section): void {
    this.current.set(section);
  }
}
