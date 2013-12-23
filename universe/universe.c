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

	universe_parse(u, "cfg/Parametres_Ressources.ini");
	universe_parse(u, "cfg/Parametres_Objets.ini");

	u->n_buildings = 49;
	kindOf_building_init(&u->buildings[ 0], g, L"Wheat farm");
	kindOf_building_init(&u->buildings[ 1], g, L"Barley farm");
	kindOf_building_init(&u->buildings[ 2], g, L"Sawmill");
	kindOf_building_init(&u->buildings[ 3], g, L"Sheepfold");
	kindOf_building_init(&u->buildings[ 4], g, L"Cattle farm");
	kindOf_building_init(&u->buildings[ 5], g, L"Butchery");
	kindOf_building_init(&u->buildings[ 6], g, L"Bakery");
	kindOf_building_init(&u->buildings[ 7], g, L"Tavern");
	kindOf_building_init(&u->buildings[ 8], g, L"Clothing shop");
	kindOf_building_init(&u->buildings[ 9], g, L"Workshop");
	kindOf_building_init(&u->buildings[10], g, L"Store");
	kindOf_building_init(&u->buildings[11], g, L"Tannery");
	kindOf_building_init(&u->buildings[12], g, L"Shoe maker");
	kindOf_building_init(&u->buildings[13], g, L"Glove maker");
	kindOf_building_init(&u->buildings[14], g, L"Wheat barn");
	kindOf_building_init(&u->buildings[15], g, L"Barley barn");
	kindOf_building_init(&u->buildings[16], g, L"Stable");
	kindOf_building_init(&u->buildings[17], g, L"Quarry");
	kindOf_building_init(&u->buildings[18], g, L"Barracks");
	kindOf_building_init(&u->buildings[19], g, L"Foundry");
	kindOf_building_init(&u->buildings[20], g, L"Smithy");
	kindOf_building_init(&u->buildings[21], g, L"Archery");
	kindOf_building_init(&u->buildings[22], g, L"Armory");
	kindOf_building_init(&u->buildings[23], g, L"Steelworks");
	kindOf_building_init(&u->buildings[24], g, L"Parchmentery");
	kindOf_building_init(&u->buildings[25], g, L"Church");
	kindOf_building_init(&u->buildings[26], g, L"Laboratory");
	kindOf_building_init(&u->buildings[27], g, L"Ether lab");
	kindOf_building_init(&u->buildings[28], g, L"Magic guild");
	kindOf_building_init(&u->buildings[29], g, L"Pyromagic school");
	kindOf_building_init(&u->buildings[30], g, L"Necropolis");
	kindOf_building_init(&u->buildings[31], g, L"Aquavitae school");
	kindOf_building_init(&u->buildings[32], g, L"Stationer");
	kindOf_building_init(&u->buildings[33], g, L"Goldsmith");
	kindOf_building_init(&u->buildings[34], g, L"Cider-house");
	kindOf_building_init(&u->buildings[35], g, L"Jeweller");
	kindOf_building_init(&u->buildings[36], g, L"Arsenal");
	kindOf_building_init(&u->buildings[37], g, L"Tectonic workshop");
	kindOf_building_init(&u->buildings[38], g, L"Chaos guild");
	kindOf_building_init(&u->buildings[39], g, L"Order guild");
	kindOf_building_init(&u->buildings[40], g, L"Elixir lab");
	kindOf_building_init(&u->buildings[41], g, L"Imperial smith");
	kindOf_building_init(&u->buildings[42], g, L"Imperial armory");
	kindOf_building_init(&u->buildings[43], g, L"Headquarters");
	kindOf_building_init(&u->buildings[44], g, L"Dragon nest");
	kindOf_building_init(&u->buildings[45], g, L"Guard tower");
	kindOf_building_init(&u->buildings[46], g, L"Market");
	kindOf_building_init(&u->buildings[47], g, L"Palace");
	kindOf_building_init(&u->buildings[48], g, L"Cantonment");

	return u;
}

void universe_exit(universe_t* u)
{
	for (int i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);

	free(u);
}

void universe_parse(universe_t* u, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open '%s'\n", filename);
		exit(1);
	}

	char*  line  = NULL;
	size_t nline = 0;
	int cur_blck = 0; // 0 = none, 1 = material, 2 = item, 3 = mine
	int cur_id = 0;
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
		// probably another section
		else if (strchr(" \t", line[0]) == NULL)
		{
			cur_blck = 0;
			continue;
		}

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
	}
	fclose(f);
}
