#include "universe.h"

#include <string.h>

#include "../util.h"

universe_t* universe_init(graphics_t* g)
{
	universe_t* u = CALLOC(universe_t, 1);

	u->n_materials = 0;
	u->materials = NULL;

	u->n_items = 0;
	u->items = NULL;

	u->n_mines = 0;
	u->mines = NULL;

	u->n_buildings = 0;
	u->buildings = NULL;

	universe_parse(u, g, "cfg/Parametres_Ressources.ini");
	universe_parse(u, g, "cfg/Parametres_Objets.ini");
	universe_parse(u, g, "cfg/Parametres_Batiments.ini");

	return u;
}

void universe_exit(universe_t* u)
{
	for (int i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);
	free(u->buildings);

	for (int i = 0; i < u->n_mines; i++)
		kindOf_mine_exit(&u->mines[i]);
	free(u->mines);

	for (int i = 0; i < u->n_items; i++)
		kindOf_item_exit(&u->items[i]);
	free(u->items);

	for (int i = 0; i < u->n_materials; i++)
		kindOf_material_exit(&u->materials[i]);
	free(u->materials);
	free(u);
}

void universe_parse(universe_t* u, graphics_t* g, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open '%s'\n", filename);
		exit(1);
	}

	char*  line  = NULL;
	size_t nline = 0;

	int cur_blck = 0; // 0 = none, 1 = material, 2 = item, 3 = mine, 4 = building
	int cur_id = 0;
	wchar_t* name = NULL;
	char* file = NULL;
	while (1)
	{
		getline(&line, &nline, f);
		if (feof(f))
			break;

		if (strncmp(line, "[Ressource_", 11) == 0) // material
		{
			cur_blck = 1;
			cur_id = atoi(line+11);
			if (cur_id > u->n_materials)
			{
				u->materials = CREALLOC(u->materials, kindOf_material_t, cur_id);
				u->n_materials = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[Objet_", 7) == 0) // item
		{
			cur_blck = 2;
			cur_id = atoi(line + 7);
			if (cur_id > u->n_items)
			{
				u->items = CREALLOC(u->items, kindOf_item_t, cur_id);
				u->n_items = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[TerrainRessource_", 18) == 0) // mine
		{
			cur_blck = 3;
			cur_id = atoi(line+18);
			if (cur_id > u->n_mines)
			{
				u->mines = CREALLOC(u->mines, kindOf_mine_t, cur_id);
				for (int i = u->n_mines; i < cur_id; i++)
					u->mines[i].id = i;
				u->n_mines = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[Batiment_", 10) == 0) // building
		{
			cur_blck = 4;
			cur_id = atoi(line+10);
			if (cur_id > u->n_buildings)
			{
				u->buildings = CREALLOC(u->buildings, kindOf_building_t, cur_id);
				memset(u->buildings + u->n_buildings, 0, (cur_id-u->n_buildings)*sizeof(kindOf_building_t));
				u->n_buildings = cur_id;
			}
			cur_id--;

			free(name);
			free(file);

			name = NULL;
			file = NULL;
			continue;
		}
		// probably another section
		else if (strchr(" \t", line[0]) == NULL)
		{
			cur_blck = 0;
			continue;
		}

		if (cur_id < 0)
			continue;

		// check if it's an affectation
		char* sep = strchr(line, '=');
		if (sep == NULL)
			continue;

		// identify the variable name
		char* var = line + strspn(line, " \t");
		var[strcspn(var, " \t=")] = 0;

		// identify the value
		sep++;
		char* val = sep + strspn(sep, " \t");
		val[strcspn(val, "\r\n")] = 0;

		if (cur_blck == 1) // material
		{
			if (strcmp(var, "Nom") == 0)
			{
				wchar_t buffer[1024];
				swprintf(buffer, 1024, L"%s", val);
				u->materials[cur_id].name = wcsdup(buffer);
			}
		}
		else if (cur_blck == 2) // item
		{
			if (strcmp(var, "Nom") == 0)
			{
				wchar_t buffer[1024];
				swprintf(buffer, 1024, L"%s", val);
				u->items[cur_id].name = wcsdup(buffer);
			}
		}
		else if (cur_blck == 3) // mine
		{
			if (strcmp(var, "Nom") == 0)
			{
				wchar_t buffer[1024];
				swprintf(buffer, 1024, L"%s", val);
				u->mines[cur_id].name = wcsdup(buffer);
			}
			else if (strcmp(var, "TypeRessource") == 0)
			{
				int id = atoi(val) - 1;
				u->mines[cur_id].material_id = id;
			}
		}
		else if (cur_blck == 4) // building
		{
			if (strcmp(var, "Nom") == 0)
			{
				wchar_t buffer[1024];
				swprintf(buffer, 1024, L"%s", val);
				name = wcsdup(buffer);
			}
			else if (strcmp(var, "Image") == 0)
			{
				file = strdup(val);
			}

			if (name && file)
			{
				kindOf_building_init(&u->buildings[cur_id], g, name, file);

				name = NULL;
				file = NULL;
			}
		}
	}
	fclose(f);
}
