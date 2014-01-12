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

#ifndef UTIL_H
#define UTIL_H

# define M_PI		3.14159265358979323846	/* pi */

static inline float max(float a, float b)
{
	return a > b ? a : b;
}

#include <stdlib.h>
#include <stdio.h>
static inline void* check_alloc(size_t n, void* ptr, const char* file, int line)
{
	void* ret = realloc(ptr, n);
	if (ret == NULL)
	{
		fprintf(stderr, "Could not allocate memory at '%s' line %i\n", file, line);
		exit(1);
	}
	return ret;
}
#define CALLOC(T,N)     ( (T*) check_alloc((N)*sizeof(T), NULL, __FILE__, __LINE__) )
#define CREALLOC(P,T,N) ( (T*) check_alloc((N)*sizeof(T), P,    __FILE__, __LINE__) )


#include <wchar.h>
#include <sys/types.h>

char*    strdup   (const char*    s);
wchar_t* wcsdup   (const wchar_t* s);
wchar_t* strdupwcs(const char*    s);

ssize_t getline(char **lineptr, size_t *n, FILE *stream);

#include <limits.h>
static inline int rnd_pick(const float* probas)
{
	float selected = (float) rand() / INT_MAX;
	int i = 0;
	while (selected > probas[i])
	{
		selected -= probas[i];
		i++;
	}
	return i;
}

#endif
