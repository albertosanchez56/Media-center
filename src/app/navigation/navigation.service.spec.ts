import { TestBed } from '@angular/core/testing';
import { NavigationService } from './navigation.service';

describe('NavigationService', () => {
  let service: NavigationService;

  beforeEach(() => {
    service = TestBed.inject(NavigationService);
  });

  it('should start on the music library', () => {
    expect(service.section()).toBe('music');
    expect(service.currentLibrary()?.name).toBe('Música');
  });

  it('should change the current library', () => {
    service.select('movie');
    expect(service.section()).toBe('movie');
    expect(service.currentLibrary()?.name).toBe('Películas');
  });

  it('should have no current library while diagnostics is shown', () => {
    service.select('diagnostics');
    expect(service.currentLibrary()).toBeNull();
  });
});
