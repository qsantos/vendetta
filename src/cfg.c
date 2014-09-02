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

#include "cfg.h"

#include <stdlib.h>

#include "string.h"
#include "mem.h"

cfg_t* cfg_new()
{
	cfg_t* cfg = CALLOC(cfg_t, 1);
	cfg->n_entries = 0;
	cfg->entries   = NULL;
	return cfg;
}

void cfg_del(cfg_t* cfg)
{
	for (size_t i = 0; i < cfg->n_entries; i++)
	{
		cfg_entry_t* e = &cfg->entries[i];
		if (e->t == E_GROUP)
			cfg_del(e->d.group);
		else
			free(e->d.str);
		free(e->key);
	}
	free(cfg->entries);
	free(cfg);
}

void cfg_put_raw(cfg_t* cfg, char* key, const char* str, cfg_etype_t t)
{
	cfg->entries = CREALLOC(cfg->entries, cfg_entry_t, cfg->n_entries+1);
	cfg_entry_t e = {t, strdup(key), {.str=strdup(str)} };
	cfg->entries[cfg->n_entries++] = e;
}

void cfg_put_bool(cfg_t* cfg, char* key, char v)
{
	cfg_put_raw(cfg, key, v ? "true" : "false", E_LITERAL);
}

void cfg_put_int(cfg_t* cfg, char* key, long int v)
{
	char buffer[32];
	snprintf(buffer, 32, "%li", v);
	cfg_put_raw(cfg, key, buffer, E_LITERAL);
}

void cfg_put_float(cfg_t* cfg, char* key, float v)
{
	// see https://randomascii.wordpress.com/2012/03/08/float
	char buffer[32];
	snprintf(buffer, 32, "%.9g", v);
	cfg_put_raw(cfg, key, buffer, E_LITERAL);
}

void cfg_put_str(cfg_t* cfg, char* key, const char* str)
{
	cfg_put_raw(cfg, key, str, E_STRING);
}

void cfg_put_group(cfg_t* cfg, char* key, cfg_t* group)
{
	if (group->n_entries == 0)
		return;
	cfg->entries = CREALLOC(cfg->entries, cfg_entry_t, cfg->n_entries+1);
	cfg_entry_t e = {E_GROUP, strdup(key), {.group=group} };
	cfg->entries[cfg->n_entries++] = e;
}

char* cfg_get_raw(cfg_t* cfg, char* key)
{
	for (size_t i = 0; i < cfg->n_entries; i++)
		if (strcmp(key, cfg->entries[i].key) == 0)
			return cfg->entries[i].d.str;
	return NULL;
}

char cfg_get_bool(cfg_t* cfg, char* key)
{
	char* raw = cfg_get_raw(cfg, key);
	if (raw == NULL || strcmp(raw, "false") == 0)
		return 0;
	return 1;
}

long int cfg_get_int(cfg_t* cfg, char* key)
{
	char* raw = cfg_get_raw(cfg, key);
	if (raw == NULL)
		return 0;
	return atol(raw);
}

float cfg_get_float(cfg_t* cfg, char* key)
{
	char* raw = cfg_get_raw(cfg, key);
	if (raw == NULL)
		return 0.f;
	return atof(raw);
}

cfg_t* cfg_get_group(cfg_t* cfg, char* key)
{
	for (size_t i = 0; i < cfg->n_entries; i++)
		if (strcmp(key, cfg->entries[i].key) == 0)
			return cfg->entries[i].d.group;
	return NULL;
}

static void fprint_indent(FILE* f, int indent)
{
	for (int i = 0; i < indent; i++)
		fprintf(f, "  ");
}
static void write_json_aux(cfg_t* cfg, FILE* f, int indent)
{
	char is_list = cfg->entries[0].key == NULL;
	if (is_list)
		fprintf(f, "[\n");
	else
		fprintf(f, "{\n");
	for (size_t i = 0; i < cfg->n_entries; i++)
	{
		cfg_entry_t* e = &cfg->entries[i];

		if (e->t == E_GROUP && e->d.group->n_entries == 0)
			continue;

		fprint_indent(f, indent+1);
		if (e->key != NULL)
			fprintf(f, "\"%s\": ", e->key);

		switch (e->t)
		{
			case E_LITERAL: fprintf(f, "%s",     e->d.str);          break;
			case E_STRING:  fprintf(f, "\"%s\"", e->d.str);          break;
			case E_GROUP:   write_json_aux(e->d.group, f, indent+1); break;
		}

		fprintf(f, ", \n");
	}
	fprint_indent(f, indent);
	if (is_list)
		fprintf(f, "]");
	else
		fprintf(f, "}");
}
void cfg_write_json(cfg_t* cfg, FILE* f)
{
	write_json_aux(cfg, f, 0);
	fprintf(f, "\n");
}
