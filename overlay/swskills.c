#include "swskills.h"

#include <math.h>

#define PANEL_N_COLS 3

void swskills_draw(game_t* g)
{
	sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {PANEL_N_COLS * 28 + 10, 310};
	wchar_t buffer[1024] = L"Skills";

	sfText_setPosition(text, pos);
	sfText_setUnicodeString(text, (sfUint32*) buffer);
	sfRenderWindow_drawText(g->g->render, text, NULL);

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
	{
		skill_t s = g->player->sskills[i];
		if (s != 1)
		{
			pos.y += 20;
			swprintf(buffer, 1024, L"%ls %i", g->u->sskills[i].name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);
		}
	}

	for (int i = 0; i < g->u->n_materials; i++)
	{
		skill_t s = g->player->mskills[i];
		if (s != 1)
		{
			pos.y += 20;
			swprintf(buffer, 1024, L"%ls %i", g->u->materials[i].skill.name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);
		}
	}

	for (int i = 0; i < g->u->n_iskills; i++)
	{
		skill_t s = g->player->iskills[i];
		if (s != 1)
		{
			pos.y += 20;
			swprintf(buffer, 1024, L"%ls %i", g->u->iskills[i].name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);
		}
	}
}

char swskills_catch(game_t* g, float x, float y)
{
	(void) g;
	(void) x;
	(void) y;
	return 0;
}
