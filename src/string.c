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

#include "string.h"

#include <stdlib.h>

char* strdup(const char* s)
{
	if (s == NULL)
		return NULL;
	size_t n = sizeof(char)*(strlen(s)+1);
	char* ret = malloc(n);
	memcpy(ret, s, n);
	return ret;
}

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

static char hex(char c)
{
	if ('a' <= c && c <= 'z')
		return c - 'a';
	if ('A' <= c && c <= 'Z')
		return c - 'A';
	else
		return c - '0';
}
static void dynstr_setchar(char** str, size_t* n, size_t i, char c)
{
	if (i >= *n)
	{
		*n = *n == 0 ? 1 : 2 * *n;
		*str = realloc(*str, *n);
	}
	(*str)[i] = c;
}
ssize_t gettoken(char** tokenptr, size_t* n, FILE* stream)
{
	char c;
	do
	{
		c = fgetc(stream);
	}
	while (c == ' ' || c == '\t' || c == '\r' || c == '\n');
	if (c == EOF)
		return -1;

	char negative = 0;
	size_t i = 0;
	dynstr_setchar(tokenptr, n, i++, c);
	if (c == '"')
	{
		char escaped = 0;
		while (1)
		{
			c = fgetc(stream);
			if (c == EOF)
			{
				fprintf(stderr, "Unmatched \"\n");
				exit(1);
			}
			if (escaped)
			{
				if (c == 'u')
				{
					unsigned int x = 0;
					x |= hex(fgetc(stream)) << 12;
					x |= hex(fgetc(stream)) <<  8;
					x |= hex(fgetc(stream)) <<  4;
					x |= hex(fgetc(stream)) <<  0;
					if (x <= 0x7f)
					{
						dynstr_setchar(tokenptr, n, i++, x);
					}
					else if (x <= 0x7ff)
					{
						dynstr_setchar(tokenptr, n, i++, 0xc0 | ((x>> 6)));
						dynstr_setchar(tokenptr, n, i++, 0x80 | ((x>> 0) & 0x3f));
					}
					else
					{
						dynstr_setchar(tokenptr, n, i++, 0xe0 | ((x>>12)));
						dynstr_setchar(tokenptr, n, i++, 0x80 | ((x>> 6) & 0x3f));
						dynstr_setchar(tokenptr, n, i++, 0x80 | ((x>> 0) & 0x3f));
					}
				}
				else if (c == 'b') dynstr_setchar(tokenptr, n, i++, '\b');
				else if (c == 'f') dynstr_setchar(tokenptr, n, i++, '\f');
				else if (c == 'n') dynstr_setchar(tokenptr, n, i++, '\n');
				else if (c == 'r') dynstr_setchar(tokenptr, n, i++, '\r');
				else if (c == 't') dynstr_setchar(tokenptr, n, i++, '\t');
				else               dynstr_setchar(tokenptr, n, i++, c);
				escaped = 0;
			}
			else if (c == '\\')
			{
				escaped = 1;
			}
			else if (c == '"')
			{
				dynstr_setchar(tokenptr, n, i++, c);
				break;
			}
			else
			{
				dynstr_setchar(tokenptr, n, i++, c);
			}
		}
	}
	else if (c == '-')
	{
		negative = 1;
		c = fgetc(stream);
		dynstr_setchar(tokenptr, n, i++, c);
	}
	else if (('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z'))
	{
		while (1)
		{
			c = fgetc(stream);
			if (!(('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z')))
				break;
			dynstr_setchar(tokenptr, n, i++, c);
		}
		ungetc(c, stream);
	}

	if ('0' <= c && c <= '9')
	{
		char decimalMark = 0;
		char exponent = 0;
		char followExponent = 0;
		while (1)
		{
			c = fgetc(stream);
			if (c == EOF)
				break;

			if (c == '.')
			{
				if (exponent)
				{
					fprintf(stderr, "Decimal mark in exponent\n");
					exit(1);
				}
				else if (decimalMark)
				{
					fprintf(stderr, "Second decimal mark in number\n");
					exit(1);
				}
				decimalMark = 1;
			}
			else if (c == 'e' || c == 'E')
			{
				if (exponent)
				{
					fprintf(stderr, "Second exponent in number\n");
					exit(1);
				}
				exponent = 1;
				followExponent = 1;
				dynstr_setchar(tokenptr, n, i++, c);
			}
			else if (c == '+' || c == '-')
			{
				if (!followExponent)
				{
					fprintf(stderr, "Invalid sign in number\n");
					exit(1);
				}
			}
			else if ('0' <= c && c <= '9')
			{
				followExponent = 0;
			}
			else
			{
				ungetc(c, stream);
				break;
			}
			dynstr_setchar(tokenptr, n, i++, c);
		}
	}
	else if (negative)
	{
		fprintf(stderr, "'-' with no number\n");
		exit(1);
	}
	dynstr_setchar(tokenptr, n, i, 0);
	return i;
}
