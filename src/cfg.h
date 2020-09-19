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

#ifndef CFG_H
#define CFG_H

typedef enum
{
	E_LITERAL, // non-string litteral
	E_STRING,  // string (add quotes)
	E_GROUP,
} cfg_etype_t;

typedef struct cfg_entry cfg_entry_t;
typedef struct cfg       cfg_t;

struct cfg_entry
{
	cfg_etype_t t;
	char* key;
	union
	{
		char*  str;
		cfg_t* group;
	} d;
};

struct cfg
{
	unsigned int n_entries;
	cfg_entry_t* entries;
};

cfg_t* cfg_new(void);
void   cfg_del(cfg_t* cfg);

void cfg_put_raw  (cfg_t* cfg, char* key, const char* str, cfg_etype_t t);
void cfg_put_bool (cfg_t* cfg, char* key, char        v);
void cfg_put_int  (cfg_t* cfg, char* key, long int    v);
void cfg_put_float(cfg_t* cfg, char* key, float       v);
void cfg_put_str  (cfg_t* cfg, char* key, const char* str);
void cfg_put_group(cfg_t* cfg, char* key, cfg_t*      group);

char*    cfg_get_raw  (cfg_t* cfg, char* key);
char     cfg_get_bool (cfg_t* cfg, char* key);
long int cfg_get_int  (cfg_t* cfg, char* key);
float    cfg_get_float(cfg_t* cfg, char* key);
cfg_t*   cfg_get_group(cfg_t* cfg, char* key);

#include <stdio.h>

void cfg_write_json(cfg_t* cfg, FILE* f);

cfg_t* cfg_load_json(FILE* f);

#endif
