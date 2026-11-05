import { TestBed } from '@angular/core/testing';
import { LIBRARIES } from '../../navigation/navigation';
import { LibraryViewComponent } from './library-view.component';

describe('LibraryViewComponent', () => {
  it('should show the title and empty state of the given library', async () => {
    await TestBed.configureTestingModule({ imports: [LibraryViewComponent] }).compileComponents();
    const fixture = TestBed.createComponent(LibraryViewComponent);
    const movies = LIBRARIES.find((library) => library.id === 'movie')!;
    fixture.componentRef.setInput('library', movies);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    expect(element.querySelector('h1')?.textContent).toContain('Películas');
    expect(element.querySelector('.library-content')?.textContent).toContain(movies.emptyTitle);
    expect(element.querySelector('button')).toBeNull();
  });
});
