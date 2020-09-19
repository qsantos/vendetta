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

// yay, again Windows compatibility
#if defined __WIN32__
typedef long off_t;
typedef long long off64_t;
#endif

#include <stdio.h>
#include <string.h>
#include <dirent.h>

char isdir(const char* path);

// executes DO for every entry in the given directory
// in DO, 'path' points to the current entry
#define FOREACH_ALL(PATH, DO) do { \
	const char* p = PATH; \
	DIR* __dir = opendir(p); \
	if (__dir == NULL) \
	{ \
		fprintf(stderr, "Cannot read '%s' folder\n", p); \
		exit(1); \
	} \
	struct dirent* __ent; \
	char path[1024]; \
	strncpy(path, p, 1024); \
	path[sizeof(path)-1] = 0; \
	while ((__ent = readdir(__dir)) != NULL) \
	{ \
		strncpy(path+strlen(p), __ent->d_name, 1024-strlen(p)); \
		path[sizeof(path)-1] = 0; \
		char __isdir = isdir(path); \
		if (__isdir) \
			path[strlen(p)+strlen(__ent->d_name)] = '/'; \
		if (__ent->d_name[0] != '.') \
		{ DO; } \
	} \
	closedir(__dir); \
	} while (0)

// same, restricted to sub-directories and file, respectively
#define FOREACH_DIR( PATH, DO) FOREACH_ALL(PATH, if( __isdir){DO;})
#define FOREACH_FILE(PATH, DO) FOREACH_ALL(PATH, if(!__isdir){DO;})

#endif
