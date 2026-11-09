import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  FolderStatus,
  ImportProgress,
  ImportRequest,
  ImportTransport,
} from './import-transport';

@Injectable({ providedIn: 'root' })
export class TauriImportTransport extends ImportTransport {
  folderStatus(): Promise<FolderStatus> {
    return invoke<FolderStatus>('import_folder_status');
  }

  start(request: ImportRequest): Promise<void> {
    return invoke<void>('import_start', { request });
  }

  cancel(): Promise<void> {
    return invoke<void>('import_cancel');
  }

  async listen(handler: (progress: ImportProgress) => void): Promise<() => void> {
    return listen<ImportProgress>('import-progress', (event) => handler(event.payload));
  }
}
