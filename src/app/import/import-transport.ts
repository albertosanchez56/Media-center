import { LibraryId } from '../navigation/navigation';

export interface FolderStatus {
  path: string;
  state: 'proposed' | 'confirmed' | 'unavailable' | 'missing';
}

export interface ImportItem {
  source: string;
  state: string;
  message: string;
}

export interface ImportProgress {
  items: ImportItem[];
  finished: boolean;
}

export interface ImportRequest {
  library: LibraryId;
  mode: 'copy' | 'move';
  kind: 'files' | 'folder';
  paths: string[];
}

export abstract class ImportTransport {
  abstract folderStatus(): Promise<FolderStatus>;
  abstract start(request: ImportRequest): Promise<void>;
  abstract cancel(): Promise<void>;
  abstract listen(handler: (progress: ImportProgress) => void): Promise<() => void>;
}
