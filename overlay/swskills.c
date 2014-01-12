
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

#include "swskills.h"

#include <math.h>

#include "../util.h"

#define PANEL_N_COLS 3

void swskills_init(swskills_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, L"Skills", 1024-SW_WIDTH*1, 0);
}

void swskills_exit(swskills_t* w)
{
	subwindow_exit(&w->w);
}

char swskills_cursor(swskills_t* w, game_t* g, int x, int y)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	return 0;
}

void swskills_draw(swskills_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {0, 0};
	wchar_t buffer[1024];

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
	{
		skill_t s = g->player->sskills[i];
		if (s != 1)
		{
			float level;
			float progress = 100 * modff(s, &level);
			swprintf(buffer, 1024, L"%ls %.0f (%.0f%%)", g->u->sskills[i].name, level, progress);

			sfText_setPosition(text, pos);
			sfText_setWString(text, buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);

			pos.y += 20;
		}
	}

	for (int i = 0; i < g->u->n_materials; i++)
	{
		skill_t s = g->player->mskills[i];
		if (s != 1)
		{
			float level;
			float progress = 100 * modff(s, &level);
			swprintf(buffer, 1024, L"%ls %.0f (%.0f%%)", g->u->materials[i].skill.name, level, progress);

			sfText_setPosition(text, pos);
			sfText_setWString(text, buffer);
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
			sfText_setWString(text, buffer);
			sfRenderWindow_drawText(g->g->render, text, NULL);

			pos.y += 20;
		}
	}

	sfRenderWindow_setView(g->g->render, g->g->overlay_view);
}

char swskills_catch(swskills_t* w, game_t* g, int x, int y, int t)
{
	(void) g;
	return subwindow_catch(&w->w, x, y, t);
}
