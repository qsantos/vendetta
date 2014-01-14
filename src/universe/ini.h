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

#ifndef CONFIG_H
#define CONFIG_H

typedef struct cfg_array   cfg_array_t;
typedef struct cfg_keyval  cfg_keyval_t;
typedef struct cfg_section cfg_section_t;
typedef struct cfg_group   cfg_group_t;
typedef struct cfg_ini     cfg_ini_t;

#include <stdio.h>

struct cfg_array
{
	size_t n;
	char** v;
};

struct cfg_keyval
{
	char* key;

	char is_array;
	union
	{
		char* val;
		cfg_array_t array;
	} v;
};

void cfg_keyval_init(cfg_keyval_t* kv, const char* key);
void cfg_keyval_exit(cfg_keyval_t* kv);

struct cfg_section
{
	char* name;

	size_t n_keyvals;
	cfg_keyval_t* keyvals;
};

void cfg_section_init (cfg_section_t* sect, const char* id);
void cfg_section_exit (cfg_section_t* sect);
void cfg_section_parse(cfg_section_t* sect, FILE* f, cfg_group_t* gr);

cfg_keyval_t* cfg_section_findKV(cfg_section_t* sect, const char* key);
void          cfg_section_saveKV(cfg_section_t* sect, char* key, char* val, cfg_group_t* gr);

int   cfg_getInt   (cfg_section_t* sect, const char* key);
float cfg_getFloat (cfg_section_t* sect, const char* key);
char* cfg_getString(cfg_section_t* sect, const char* key);

cfg_array_t* cfg_getArray(cfg_section_t* sect, const char* key);

struct cfg_group
{
	char* name;

	size_t n_sections;
	cfg_section_t* sections;
};

void cfg_group_init (cfg_group_t* gr, const char* name);
void cfg_group_exit (cfg_group_t* gr);
void cfg_group_parse(cfg_group_t* gr, FILE* f, const char* id);

struct cfg_ini
{
	size_t n_groups;
	cfg_group_t* groups;
};

void cfg_ini_init (cfg_ini_t* ini);
void cfg_ini_exit (cfg_ini_t* ini);
void cfg_ini_parse(cfg_ini_t* ini, const char* filename);

cfg_group_t* cfg_ini_group(cfg_ini_t* ini, const char* name);

#endif
