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

#include "ini.h"

#include <stdlib.h>

#include "../mem.h"
#include "../string.h"

void cfg_keyval_init(cfg_keyval_t* kv, const char* key)
{
	memset(kv, 0, sizeof(cfg_keyval_t));
	kv->key = strdup(key);
}

void cfg_keyval_exit(cfg_keyval_t* kv)
{
	if (kv->is_array)
	{
		for (size_t i = 0; i < kv->v.array.n; i++)
			free(kv->v.array.v[i]);
	}
	else
	{
		free(kv->v.val);
	}
	free(kv->key);
}

void cfg_section_init(cfg_section_t* sect, const char* id)
{
	*sect = (cfg_section_t){strdup(id), 0, NULL};
}

void cfg_section_exit(cfg_section_t* sect)
{
	for (size_t i = 0; i < sect->n_keyvals; i++)
		cfg_keyval_exit(&sect->keyvals[i]);
	free(sect->keyvals);
	free(sect->name);
}

void cfg_section_parse(cfg_section_t* sect, FILE* f, cfg_group_t* gr)
{
	// for simplicity, this parsing might have started reading the next section
	// to avoid mangling the source, we just rewing the current file offset
	long prev_line = ftell(f);;

	char* line = NULL;
	size_t n_line = 0;
	while (getline(&line, &n_line, f) > 0)
	{
		// check if it's an affectation
		char* sep = strchr(line, '=');
		if (sep == NULL)
			break;

		// identify the key name
		char* key = line + strspn(line, " \t");
		key[strcspn(key, " \t=")] = 0;

		// identify the value
		sep++;
		char* val = sep + strspn(sep, " \t\"");
		val[strcspn(val, "\r\n\"")] = 0;

		cfg_section_saveKV(sect, key, val, gr);

		prev_line = ftell(f);
	}
	fseek(f, prev_line, SEEK_SET);
	free(line);
}

cfg_keyval_t* cfg_section_findKV(cfg_section_t* sect, const char* key)
{
	// find keyval
	size_t i = 0;
	for (; i < sect->n_keyvals && strcmp(key, sect->keyvals[i].key) != 0; i++);

	if (i == sect->n_keyvals)
	{
		sect->n_keyvals++;
		sect->keyvals = CREALLOC(sect->keyvals, cfg_keyval_t, sect->n_keyvals);
		cfg_keyval_init(&sect->keyvals[i], key);
	}

	return &sect->keyvals[i];
}

void cfg_section_saveKV(cfg_section_t* sect, char* key, char* val, cfg_group_t* gr)
{
	char* idx = strchr(key, '(');
	if (idx != NULL) // array variable
	{
		*idx = 0;
		size_t i = strtol(idx+1, NULL, 0);
		if (i == 0)
		{
			fprintf(stderr, "Arrays are indexed starting at 1\n");
			exit(1);
		}
		i--;
		cfg_keyval_t* kv = cfg_section_findKV(sect, key);
		if (!kv->is_array && kv->v.val != NULL)
		{
			fprintf(stderr, "Simple variable '%s' used as array variable in '%s_%s'\n",
					key, gr->name, sect->name);
			exit(1);
		}
		kv->is_array = 1;
		if (i >= kv->v.array.n)
		{
			kv->v.array.v = CREALLOC(kv->v.array.v, char*, i+1);
			memset(kv->v.array.v + kv->v.array.n, 0, sizeof(char*)*(i+1-kv->v.array.n));
			kv->v.array.n = i+1;
		}
		free(kv->v.array.v[i]);
		kv->v.array.v[i] = strdup(val);
	}
	else // simple variable
	{
		cfg_keyval_t* kv = cfg_section_findKV(sect, key);
		if (kv->is_array)
		{
			fprintf(stderr, "Array variable '%s' used as simple variable in '%s_%s'\n",
					key, gr->name, sect->name);
			exit(1);
		}
		kv->v.val = strdup(val);
	}
}

static char* kv_find(cfg_section_t* sect, const char* key)
{
	for (size_t i = 0; i < sect->n_keyvals; i++)
	{
		cfg_keyval_t* kv = &sect->keyvals[i];
		if (strcmp(key, kv->key) != 0)
			continue;

		if (kv->is_array)
		{
			fprintf(stderr, "Variable '%s' is not expected to be an array\n",
				kv->key);
			exit(1);
		}
		return kv->v.val;
	}
	return NULL;
}

int cfg_getInt(cfg_section_t* sect, const char* key)
{
	char* val = kv_find(sect, key);
	if (val == NULL)
		return 0;
	return strtol(val, NULL, 0);
}

float cfg_getFloat(cfg_section_t* sect, const char* key)
{
	char* val = kv_find(sect, key);
	if (val == NULL)
		return 0;
	return atof(val);
}

char* cfg_getString(cfg_section_t* sect, const char* key)
{
	char* val = kv_find(sect, key);
	if (val == NULL)
		return NULL;
	return strdup(val);
}

cfg_array_t* cfg_getArray(cfg_section_t* sect, const char* key)
{
	for (size_t i = 0; i < sect->n_keyvals; i++)
	{
		cfg_keyval_t* kv = &sect->keyvals[i];
		if (strcmp(key, kv->key) != 0)
			continue;

		if (!kv->is_array)
		{
			fprintf(stderr, "Variable '%s' is expected to be an array\n",
				kv->key);
			exit(1);
		}
		return &kv->v.array;
	}
	return NULL;
}

void cfg_group_init(cfg_group_t* gr, const char* name)
{
	*gr = (cfg_group_t){strdup(name), 0, NULL};
}

void cfg_group_exit(cfg_group_t* gr)
{
	for (size_t i = 0; i < gr->n_sections; i++)
		cfg_section_exit(&gr->sections[i]);
	free(gr->sections);
	free(gr->name);
}

void cfg_group_parse(cfg_group_t* gr, FILE* f, const char* id)
{
	// find section with this id
	size_t i = 0;
	for (; i < gr->n_sections && strcmp(id, gr->sections[i].name) != 0; i++);

	if (i == gr->n_sections)
	{
		gr->n_sections++;
		gr->sections = CREALLOC(gr->sections, cfg_section_t, gr->n_sections);
		cfg_section_init(&gr->sections[i], id);
	}

	cfg_section_parse(&gr->sections[i], f, gr);
}

void cfg_ini_init(cfg_ini_t* ini)
{
	*ini = (cfg_ini_t){0, NULL};
}

void cfg_ini_exit(cfg_ini_t* ini)
{
	for (size_t i = 0; i < ini->n_groups; i++)
		cfg_group_exit(&ini->groups[i]);
	free(ini->groups);
}

void cfg_ini_parse(cfg_ini_t* ini, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open '%s'\n", filename);
		exit(1);
	}

	fprintf(stderr, "Parsing file '%s'\n", filename);

	char* line = NULL;
	size_t n_line = 0;
	while (getline(&line, &n_line, f) > 0)
	{
		if (line[0] != '[')
			continue;

		const char* group = strtok(line+1, "_");
		const char* id    = strtok(NULL,   "]");

		if (id == NULL)
			continue;

		// find group of this name
		size_t i = 0;
		for (; i < ini->n_groups && strcmp(group, ini->groups[i].name) != 0; i++);

		if (i == ini->n_groups)
		{
			ini->n_groups++;
			ini->groups = CREALLOC(ini->groups, cfg_group_t, ini->n_groups);
			cfg_group_init(&ini->groups[i], group);
		}

		cfg_group_parse(&ini->groups[i], f, id);
	}

	free(line);
	fclose(f);
}

cfg_group_t* cfg_ini_group(cfg_ini_t* ini, const char* name)
{
	for (size_t i = 0; i < ini->n_groups; i++)
		if (strcmp(name, ini->groups[i].name) == 0)
			return &ini->groups[i];
	return NULL;
}
