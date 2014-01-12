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

#include "util.h"

#include <string.h>

char* strdup(const char* s)
{
	size_t n = sizeof(char)*(strlen(s)+1);
	char* ret = malloc(n);
	memcpy(ret, s, n);
	return ret;
}

wchar_t* wcsdup(const wchar_t* s)
{
	size_t n = sizeof(wchar_t)*(wcslen(s)+1);
	wchar_t* ret = malloc(n);
	memcpy(ret, s, n);
	return ret;
}

#ifdef __WIN32__
#include <windows.h>
#include <Winnls.h>
wchar_t* strdupwcs(const char* s)
{
	wchar_t buffer[1024];
	MultiByteToWideChar(CP_UTF8, 0, s, -1, buffer, 1024);
	return wcsdup(buffer);
}
#else
wchar_t* strdupwcs(const char* s)
{
	wchar_t buffer[1024];
	mbstowcs(buffer, s, 1024);
	return wcsdup(buffer);
}
#endif

ssize_t getline(char** lineptr, size_t* n, FILE* stream)
{
	size_t i = 0;
	while (1)
	{
		int c = fgetc(stream);
		if (c < 0)
			return -1;
		if (i == *n)
		{
			*n = *n == 0 ? 128 : 2* *n;
			*lineptr = realloc(*lineptr, *n);
		}
		(*lineptr)[i++] = c;
		if (c == '\n')
			break;
	}
	if (i == *n)
	{
		*n = *n == 0 ? 128 : 2* *n;
		*lineptr = realloc(*lineptr, *n);
	}
	(*lineptr)[i] = 0;
	return i;
}
