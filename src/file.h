/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef FILE_H
#define FILE_H

#if defined __WIN32__ && ! defined __MINGW32__
#define off64_t long long
#else
enum
{
	DT_UNKNOWN = 0,
	DT_FIFO = 1,
	DT_CHR = 2,
	DT_DIR = 4,
	DT_BLK = 6,
	DT_REG = 8,
	DT_LNK = 10,
	DT_SOCK = 12,
	DT_WHT = 14
};
#endif

#include <stdio.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>

char isdir(const char* path);

#define FOREACH_FILE(PATH, DO) do { \
	DIR* __dir = opendir(PATH); \
	if (__dir == NULL) \
	{ \
		fprintf(stderr, "Cannot read '%s' folder\n", PATH); \
		exit(1); \
	} \
	struct dirent* __ent; \
	char path[1024] = PATH; \
	while ((__ent = readdir(__dir)) != NULL) \
	{ \
		strncpy(path+strlen(PATH), __ent->d_name, 1024-strlen(PATH)); \
		if (!isdir(path)) { DO }; \
	} \
	closedir(__dir); \
	} while (0)

#endif
