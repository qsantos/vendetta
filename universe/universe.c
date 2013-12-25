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

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		kindOf_skill_init(&u->sskills[i]);

	u->n_iskills = 0;
	u->iskills = NULL;

	// parse configuration files
	u->harvestRates = NULL;
	u->rawMaterials = NULL;
	u->rawAmounts   = NULL;
	u->item_req     = NULL;

	universe_parse(u, g, "cfg/Parametres_Ressources.ini");
	universe_parse(u, g, "cfg/Parametres_CompetencesObjets.ini");
	universe_parse(u, g, "cfg/Parametres_Objets.ini");
	universe_parse(u, g, "cfg/Parametres_Batiments.ini");
	universe_parse(u, g, "cfg/Competences_Speciales.ini");

	for (int i = 0; i < u->n_items; i++)
		components_exit(&u->item_req[i]);
	free(u->item_req);
	free(u->rawAmounts);
	free(u->rawMaterials);
	free(u->harvestRates);

	return u;
}

void universe_exit(universe_t* u)
{
	for (int i = 0; i < u->n_iskills; i++)
		kindOf_skill_exit(&u->iskills[i]);
	free(u->iskills);

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		kindOf_skill_exit(&u->sskills[i]);

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

	// 0 = none, 1 = material, 2 = item, 3 = mine,
	// 4 = building, 5 = item skill
	int cur_blck = 0;

	int cur_id = 0;

	char* image_file = NULL;
	int n_sprites = 0;
	char* button_file = NULL;
	int button_index = -1;

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
				u->harvestRates = CREALLOC(u->harvestRates, float, cur_id);
				u->rawMaterials = CREALLOC(u->rawMaterials, int,   cur_id);
				u->rawAmounts   = CREALLOC(u->rawAmounts,   float, cur_id);
				for (int i = u->n_materials; i < cur_id; i++)
					kindOf_material_init(&u->materials[i]);
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
				u->items    = CREALLOC(u->items,    kindOf_item_t, cur_id);
				u->item_req = CREALLOC(u->item_req, components_t,  cur_id);
				for (int i = u->n_items; i < cur_id; i++)
				{
					kindOf_item_init(&u->items[i]);
					components_init(&u->item_req[i]);
				}
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
				for (int i = u->n_buildings; i < cur_id; i++)
					kindOf_building_init(&u->buildings[i]);
				u->n_buildings = cur_id;
			}
			cur_id--;

			free(image_file);
			free(button_file);

			image_file = NULL;
			n_sprites = 0;
			button_file = NULL;
			button_index = -1;
			continue;
		}
		else if (strncmp(line, "[CompetenceObjet_", 17) == 0) // item skill
		{
			cur_blck = 5;
			cur_id = atoi(line+17);
			if (cur_id > u->n_iskills)
			{
				u->iskills = CREALLOC(u->iskills, kindOf_skill_t, cur_id);
				for (int i = u->n_iskills; i < cur_id; i++)
					kindOf_skill_init(&u->iskills[i]);
				u->n_iskills = cur_id;
			}
			cur_id--;
			continue;
		}
		// probably another section
		else if (line[0] == '[')
		{
			cur_blck = 0;
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
		char* val = sep + strspn(sep, " \t\"");
		val[strcspn(val, "\r\n\"")] = 0;

		if (cur_blck == 0) // global
		{
			if (strncmp(var, "CompetencesSpeciales", 20) == 0)
			{
				int id = atoi(var+20) - 1;
				u->sskills[id].name = strdupwcs(val);
			}
		}

		if (cur_id < 0)
			continue;

		if (cur_blck == 1) // material
		{
			if (strcmp(var, "Nom") == 0)
			{
				u->materials[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "VitesseExtraction") == 0)
			{
				u->harvestRates[cur_id] = atof(val);
			}
			else if (strcmp(var, "TypeMatierePremiere") == 0)
			{
				u->rawMaterials[cur_id] = atoi(val) - 1;
			}
			else if (strcmp(var, "QuantiteMatierePremiere") == 0)
			{
				u->rawAmounts[cur_id] = atof(val);
			}
			else if (strcmp(var, "NomCompetence") == 0)
			{
				u->materials[cur_id].skill.name = strdupwcs(val);
			}
		}
		else if (cur_blck == 2) // item
		{
			if (strcmp(var, "Nom") == 0 || strcmp(var, "NomFR") == 0)
			{
				u->items[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "Competence") == 0)
			{
				u->items[cur_id].skill = atoi(val) - 1;
			}
			else if (strncmp(var, "PrixRessources(", 15) == 0)
			{
				int mat_id = atoi(var + 15) - 1;
				float amount = atof(val);
				components_material(&u->item_req[cur_id], mat_id, amount);
			}
			else if (strcmp(var, "DureeFabrication") == 0)
			{
				u->item_req[cur_id].rate = 100.f / atoi(val);
			}
		}
		else if (cur_blck == 3) // mine
		{
			if (strcmp(var, "Nom") == 0)
			{
				kindOf_mine_init(&u->mines[cur_id], strdupwcs(val));
			}
			else if (strcmp(var, "TypeRessource") == 0)
			{
				int id = atoi(val) - 1;
				u->mines[cur_id].harvest.rate = u->harvestRates[id];
				components_material(&u->mines[cur_id].harvest, id, 1);
			}
		}
		else if (cur_blck == 4) // building
		{
			if (strcmp(var, "Nom") == 0)
			{
				u->buildings[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "Image") == 0)
			{
				image_file = strdup(val);
			}
			else if (strcmp(var, "ImageBouton") == 0)
			{
				button_file = strdup(val);
			}
			else if (strcmp(var, "SpriteBoutonIndex") == 0)
			{
				button_index = atoi(val) - 1;
			}
			else if (strcmp(var, "MaxVie") == 0)
			{
				u->buildings[cur_id].build_time = atof(val) / 100.;
			}
			else if (strncmp(var, "PrixRessources(", 15) == 0)
			{
				int mat_id = atoi(var + 15) - 1;
				float amount = atof(val);
				components_material(&u->buildings[cur_id].build_req, mat_id, amount);
			}
			else if (strcmp(var, "RessourceFabrique") == 0)
			{
				int id = atoi(val) - 1;
				if (id >= 0)
				{
					if (u->rawMaterials[id] >= 0)
						components_material(&u->buildings[cur_id].make_req, u->rawMaterials[id], u->rawAmounts[id]);
					components_material(&u->buildings[cur_id].make_res, id, 1);
				}
			}
			else if (strncmp(var, "ObjetsFabriques(", 16) == 0)
			{
				int id = atoi(val) - 1;

				kindOf_building_t* b = &u->buildings[cur_id];
				kindOf_building_newItem(b);
				components_copy(&b->item_req[b->item_n-1], &u->item_req[id]);
				components_item(&b->item_res[b->item_n-1], id, 1);
			}
			else if (strcmp(var, "NombreEtapeFabrication") == 0)
			{
				n_sprites = atoi(val);
			}

			if (image_file && n_sprites)
			{
				kindOf_building_sprite(&u->buildings[cur_id], g, image_file, n_sprites+1);

				image_file = NULL;
				n_sprites = 0;
			}
			if (button_file && button_index >= 0)
			{
				kindOf_building_button(&u->buildings[cur_id], g, button_file, button_index);

				button_file = NULL;
				button_index = 0;
			}
		}
		else if (cur_blck == 5) // item skill
		{
			if (strcmp(var, "NomFR") == 0)
			{
				u->iskills[cur_id].name = strdupwcs(val);
			}
		}
	}

	fclose(f);
}
