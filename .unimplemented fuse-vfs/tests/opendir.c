#include <dirent.h>
#include <stdio.h>

int main() {
  DIR *dir;
  struct dirent *entry;

  dir = opendir("mnt");
  if (dir == NULL) {
    perror("Unable to open directory");
    return 1;
  }

  int count = 0;
  while ((entry = readdir(dir)) != NULL) {
    count++;
  }

  if (count == 0) {
    perror("No files found in mnt");
    return 1;
  }

  closedir(dir);
  return 0;
}
