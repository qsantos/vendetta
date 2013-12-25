#include "swskills.h"

#include <math.h>

#define PANEL_N_COLS 3

void swskills_init(swskills_t* w)
{
	subwindow_init(&w->w, L"Skills", 1024-SW_WIDTH*1, 0);
}

void swskills_exit(swskills_t* w)
{
	subwindow_exit(&w->w);
}

void swskills_draw(swskills_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 50};
	wchar_t buffer[1024];

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
	{
		skill_t s = g->player->sskills[i];
		if (s != 1)
		{
			swprintf(buffer, 1024, L"%ls %i", g->u->sskills[i].name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);

			pos.y += 20;
		}
	}

	for (int i = 0; i < g->u->n_materials; i++)
	{
		skill_t s = g->player->mskills[i];
		if (s != 1)
		{
			swprintf(buffer, 1024, L"%ls %i", g->u->materials[i].skill.name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);

			pos.y += 20;
		}
	}

	for (int i = 0; i < g->u->n_iskills; i++)
	{
		skill_t s = g->player->iskills[i];
		if (s != 1)
		{
			swprintf(buffer, 1024, L"%ls %i", g->u->iskills[i].name, (int)floor(s*100));

			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);

			pos.y += 20;
		}
	}
}

char swskills_catch(swskills_t* w, game_t* g, float x, float y)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	return 0;
}
